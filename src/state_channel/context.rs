use borsh::BorshSerialize;
use borsh_derive::{BorshDeserialize, BorshSerialize};
use colored::Colorize;
use libp2p::{futures::io, PeerId};
use log::info;

use crate::p2p::{read, Message, MessageType};
#[derive(Debug)]
pub struct ChannelLedgerIxs {
    pub peer_id: PeerId,
    pub ixs: Vec<StateChanIx>,
}

impl ChannelLedgerIxs {
    pub fn new(peer_id: &PeerId) -> Self {
        Self {
            peer_id: peer_id.clone(),
            ixs: Vec::new(),
        }
    }

    pub fn push_ix(&mut self, ix: StateChanIx) {
        info!("Pushing instruction to ledger : {:?}", ix);
        self.ixs.push(ix);
    }

    pub fn pop_ix(&mut self) -> Option<StateChanIx> {
        self.ixs.pop()
    }

    pub fn len(&self) -> usize {
        self.ixs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ixs.is_empty()
    }

    pub fn show_ixs(&self) {
        println!("{}", "State Channel Instructions:".bold());
        for (i, ix) in self.ixs.iter().enumerate() {
            match ix {
                StateChanIx::NativeSOLTransfer(_) => {
                    println!("{}", format!("{}.NativeSOLTransfer", i + 1).bold().blue());
                }
                StateChanIx::SPLTokenTransfer(_) => {
                    println!("{}", format!("{}.SPLTokenTransfer", i + 1).bold().blue());
                }
                StateChanIx::None => {
                    println!("Invalid Ix")
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum StateChanState {
    Open { is_settled: bool },
    Closed,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct NativeSOLTransfer {
    pub peer_id: String,
    pub amount: u64,
    pub recipient: String,
}

impl NativeSOLTransfer {
    pub fn read_data_from_cli(peer_id: &PeerId) -> Result<Self, io::Error> {
        let mut buf = String::new();
        println!("{}", "Enter the amount of SOL to transfer:".green().bold());
        read(&mut buf)?;
        let amount = buf.trim().parse::<u64>().unwrap();
        buf.clear();
        println!("{}", "Enter the recipient address:".green().bold());
        read(&mut buf)?;
        let recipient = buf.trim().to_string();
        Ok(Self {
            peer_id: peer_id.to_string(),
            amount,
            recipient,
        })
    }
}

impl SPLTokenTransfer {
    pub fn read_data_from_cli(peer_id: &PeerId) -> Result<Self, io::Error> {
        let mut buf = String::new();
        println!("{}", "Enter the token mint:".green().bold());
        read(&mut buf)?;
        let token_mint = buf.trim().to_string();
        buf.clear();
        println!(
            "{}",
            "Enter the amount of token to transfer:".green().bold()
        );
        read(&mut buf)?;
        let amount = buf.trim().parse::<u64>().unwrap();
        buf.clear();
        println!("{}", "Enter the recipient address:".green().bold());
        read(&mut buf)?;
        let recipient = buf.trim().to_string();
        Ok(Self {
            peer_id: peer_id.to_string(),
            token_mint,
            amount,
            recipient,
        })
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct SPLTokenTransfer {
    pub peer_id: String,
    pub token_mint: String,
    pub amount: u64,
    pub recipient: String,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub enum StateChanIx {
    NativeSOLTransfer(NativeSOLTransfer),
    SPLTokenTransfer(SPLTokenTransfer),
    None,
}

impl StateChanIx {
    pub fn show_and_read_ixs(peer_id: &PeerId, ledger: &mut ChannelLedgerIxs) -> Option<Message> {
        println!();
        println!("{}", "State Channel Instructions:".bold());
        println!("{}", "1.NativeSOLTransfer".bold().cyan());
        println!("{}", "2.SPLTokenTransfer".bold().bright_purple());
        println!("{}", "Choose Ix:".bold());
        println!();

        let mut buf = String::new();
        read(&mut buf).unwrap();
        let ix = buf.trim().parse::<u8>().unwrap();
        let read_ix = match ix {
            1 => {
                let native_sol_transfer = NativeSOLTransfer::read_data_from_cli(peer_id).unwrap();
                StateChanIx::NativeSOLTransfer(native_sol_transfer)
            }
            2 => {
                let spl_token_transfer = SPLTokenTransfer::read_data_from_cli(peer_id).unwrap();
                StateChanIx::SPLTokenTransfer(spl_token_transfer)
            }
            _ => return None,
        };

        let mut ix_data: Vec<u8> = Vec::new();

        ledger.push_ix(read_ix.clone());
        read_ix.serialize(&mut ix_data).unwrap();

        Some(Message::new(MessageType::PushInstruction, ix_data))
    }
}
