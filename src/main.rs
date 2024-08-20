mod p2p;
mod state_channel;

use borsh::{BorshDeserialize, BorshSerialize};
use colored::Colorize;
use libp2p::futures::StreamExt;
use libp2p::{gossipsub, mdns, noise, swarm::SwarmEvent, tcp, yamux};
use libp2p::{PeerId, SwarmBuilder};
use log::info;
use p2p::{Message, MessageType, P2PConnectionsInfo, P2PNetBehaviour, P2PNetBehaviourEvent};
use state_channel::*;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;
use tokio::io::AsyncBufReadExt;
use tokio::{io, select};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::builder()
        .format_module_path(false)
        .format_timestamp_micros()
        .init();

    let mut p2p_conns = P2PConnectionsInfo::new();
    let p2p_keypair = libp2p::identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from_public_key(&p2p_keypair.public());
    p2p_conns.set_local_peer_id(local_peer_id.clone());
    let mut p2p_swarm = SwarmBuilder::with_existing_identity(p2p_keypair)
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_quic()
        .with_behaviour(|key| {
            // To content-address message, we can take the hash of message and use it as an ID.
            let message_id_fn = |message: &gossipsub::Message| {
                let mut hasher = DefaultHasher::new();
                message.data.hash(&mut hasher);
                gossipsub::MessageId::from(hasher.finish().to_string())
            };

            // Set a custom gossipsub configuration
            let gossipsub_config = gossipsub::ConfigBuilder::default()
                .heartbeat_interval(Duration::from_secs(10)) // This is set to aid debugging by not cluttering the log space
                .validation_mode(gossipsub::ValidationMode::Strict) // This sets the kind of message validation. The default is Strict (enforce message signing)
                .message_id_fn(message_id_fn) // content-address messages. No two messages of the same content will be propagated.
                .build()
                .map_err(|msg| io::Error::new(io::ErrorKind::Other, msg))?; // Temporary hack because `build` does not return a proper `std::error::Error`.

            // build a gossipsub network behaviour
            let gossipsub = gossipsub::Behaviour::new(
                gossipsub::MessageAuthenticity::Signed(key.clone()),
                gossipsub_config,
            )?;

            let mdns =
                mdns::tokio::Behaviour::new(mdns::Config::default(), key.public().to_peer_id())?;
            Ok(P2PNetBehaviour { gossipsub, mdns })
        })?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    // Create a Gossipsub topic
    let topic = gossipsub::IdentTopic::new("solana-payment-channel");
    // subscribes to our topic
    p2p_swarm.behaviour_mut().gossipsub.subscribe(&topic)?;

    // Listen on all interfaces and whatever port the OS assigns
    // p2p_swarm.listen_on("/ip4/0.0.0.0/udp/0/quic-v1".parse()?)?;
    p2p_swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    info!("Listening on tcp and quic interfaces");
    info!(
        "Local peer id: {}",
        p2p_swarm.local_peer_id().to_base58().bold().cyan()
    );
    let mut buf = String::new();
    let mut stdin = io::BufReader::new(io::stdin());
    let peer_id = p2p_swarm.local_peer_id().clone();

    let mut ledger = state_channel::ChannelLedgerIxs::new(&peer_id);
    let mut show_ixs = true;

    loop {
        if show_ixs {
            p2p::show_p2p_options();
            show_ixs = false;
        }

        select! {
            Ok(_) = stdin.read_line(&mut buf) => {
                let read_command_res =  buf.trim().parse::<u8>().map_err(|_| io::Error::new(io::ErrorKind::Other, "Invalid command"));
                let command = match read_command_res{
                    Ok(command) => command,
                    Err(e) => {
                        println!("Error reading command: {:?}", e);
                        show_ixs = true;
                        continue;
                    }
                };


                let message = match command {

                    1 => {
                        Some(state_channel::StateChanIx::show_and_read_ixs(&local_peer_id,&mut ledger).unwrap())
                        },
                    2 => {
                        let ixs_len = ledger.len();
                        Some(Message::new(MessageType::AnnounceIxsLen, ixs_len.to_le_bytes().to_vec()))
                        }
                    _ =>{
                        None
                    }
                    };

                let mut message_bytes:Vec<u8> = Vec::new();
                if let Some(message) = message {
                    message.serialize(&mut message_bytes)?;
                    info!("Sending message: {:?}",message);
                    let _publish_res = p2p_swarm.behaviour_mut().gossipsub.publish(topic.clone(),message_bytes);
                }
                else{
                    info!("Invalid Message");
                }
                buf.clear();
                show_ixs = true;
            }
            event = p2p_swarm.select_next_some() => match event {
                SwarmEvent::NewListenAddr { address, .. } => {
                    info!("Listening on {}", address.to_string().bold().green());
                },
                SwarmEvent::Behaviour(P2PNetBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                    for (peer_id, _multiaddr) in list {
                        // info!("mDNS discovered a new peer: {peer_id}");
                        p2p_swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                    }
                },
                SwarmEvent::Behaviour(P2PNetBehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
                    for (peer_id, _multiaddr) in list {
                        // info!("mDNS discover peer has expired: {peer_id}");
                        p2p_swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer_id);
                    }
                },
                SwarmEvent::Behaviour(P2PNetBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                    propagation_source: peer_id,
                    message_id: id,
                    message,
                })) =>{
                    info!("Received message from peer: {} with message id : {}",peer_id.to_base58().bold().cyan(),id.to_string().bold().cyan());
                    match Message::try_from_slice(message.data.as_slice()){
                        Ok(msg) => {
                            info!("Deserialized message: {:?}", msg);
                            match msg.message_type {
                                MessageType::AnnounceIxsLen => {
                                    let ixs_len = u64::from_le_bytes(msg.message_data.unwrap().as_slice().try_into().unwrap());
                                    info!("Ixs len: {}",ixs_len);
                                },
                                MessageType::PushInstruction => {
                                    let ix = StateChanIx::try_from_slice(msg.message_data.unwrap().as_slice())?;
                                    ledger.push_ix(ix);
                                },
                                _ => {}
                            }
                        },
                        Err(e) => {
                            info!("Error deserializing message: {:?}",e);
                        }
                    }
                },

                SwarmEvent::Behaviour(P2PNetBehaviourEvent::Gossipsub(gossipsub::Event::Subscribed { peer_id, topic })) => {
                    info!("Subscribed to topic: {} with peer: {}",topic.to_string().bold().cyan(),peer_id.to_base58().bold().cyan());
                },

                SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                    info!("Connection established with peer: {}",peer_id.to_base58().bold().cyan());
                    p2p_conns.push_peer(peer_id);
                },

                SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                    info!("Connection with peer: {peer_id} closed: {}",cause.unwrap().to_string().bold().cyan());
                    p2p_conns.remove_peer(&peer_id);

                },
                _ => { }

            }
        }
    }
}
