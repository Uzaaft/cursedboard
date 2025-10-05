pub mod clipboard;
pub mod cli;
pub mod config;
pub mod connection;
pub mod discovery;
mod duration_serde;
pub mod instance;
pub mod network;
pub mod protocol;
pub mod simple_config_loader;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardMessage {
    pub content: String,
    pub timestamp: u64,
}

impl ClipboardMessage {
    pub fn new(content: String) -> Self {
        Self {
            content,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(self)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(bytes)
    }
}

pub fn encode_message(msg: &ClipboardMessage) -> Result<Vec<u8>, bincode::Error> {
    let serialized = msg.to_bytes()?;
    let len = serialized.len() as u64;
    let mut result = Vec::with_capacity(8 + serialized.len());
    result.extend_from_slice(&len.to_le_bytes());
    result.extend_from_slice(&serialized);
    Ok(result)
}

pub fn decode_message_length(bytes: &[u8; 8]) -> u64 {
    u64::from_le_bytes(*bytes)
}
