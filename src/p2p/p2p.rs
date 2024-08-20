use std::time::Instant;

use borsh_derive::{BorshDeserialize, BorshSerialize};
use libp2p::swarm::NetworkBehaviour;
use libp2p::{gossipsub, PeerId};

use super::get_unix_timestamp_secs;

type Mdns = libp2p::mdns::tokio::Behaviour;

#[derive(NetworkBehaviour)]
pub struct P2PNetBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub mdns: Mdns,
}

#[derive(Debug)]
pub struct P2PConnectionsInfo {
    pub local_peer_id: Option<PeerId>,
    pub connected_peers: Option<Vec<PeerId>>,
}

impl P2PConnectionsInfo {
    pub fn new() -> Self {
        Self {
            local_peer_id: None,
            connected_peers: None,
        }
    }

    pub fn set_local_peer_id(&mut self, peer_id: PeerId) {
        self.local_peer_id = Some(peer_id);
    }

    pub fn push_peer(&mut self, peer_id: PeerId) {
        if self.connected_peers.is_none() {
            self.connected_peers = Some(Vec::new());
            self.connected_peers.as_mut().unwrap().push(peer_id);
        } else {
            self.connected_peers.as_mut().unwrap().push(peer_id);
        }
    }

    pub fn remove_peer(&mut self, peer_id: &PeerId) {
        if self.connected_peers.is_some() {
            let index = self
                .connected_peers
                .as_ref()
                .unwrap()
                .iter()
                .position(|x| x == peer_id);
            if let Some(index) = index {
                self.connected_peers.as_mut().unwrap().remove(index);
            }
        }
    }
}

#[derive(Debug, BorshDeserialize, BorshSerialize)]
pub enum MessageType {
    PushInstruction,
    AnnounceIxsLen,
    None,
}

#[derive(Debug, BorshDeserialize, BorshSerialize)]
pub struct Message {
    pub message_type: MessageType,
    pub message_data: Option<Vec<u8>>,
    pub timestamp: u64,
}

impl Message {
    pub fn new(message_type: MessageType, message_data: Vec<u8>) -> Self {
        Self {
            message_type,
            message_data: Some(message_data),
            timestamp: get_unix_timestamp_secs(),
        }
    }

    pub fn default() -> Self {
        Self {
            message_type: MessageType::None,
            message_data: None,
            timestamp: get_unix_timestamp_secs(),
        }
    }
}
