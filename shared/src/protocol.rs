use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    Hello(HelloMessage),
    ClipboardUpdate(ClipboardUpdateMessage),
    Keepalive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloMessage {
    pub instance_id: Uuid,
    pub device_name: String,
    pub version: String,
    pub features: Vec<String>,
    pub group: String,
    pub mac: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardUpdateMessage {
    pub content: String,
    pub timestamp: u64,
}

impl Message {
    pub fn to_bytes(&self) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(self)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(bytes)
    }
}

pub fn encode_message(msg: &Message) -> Result<Vec<u8>, bincode::Error> {
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

impl HelloMessage {
    pub fn new(
        instance_id: Uuid,
        device_name: String,
        group: String,
        psk: Option<&str>,
    ) -> Self {
        let version = env!("CARGO_PKG_VERSION").to_string();
        let features = vec!["text".to_string()];
        
        let mac = psk.map(|key| {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            
            let mut hasher = DefaultHasher::new();
            instance_id.hash(&mut hasher);
            device_name.hash(&mut hasher);
            group.hash(&mut hasher);
            key.hash(&mut hasher);
            
            hasher.finish().to_le_bytes().to_vec()
        });

        HelloMessage {
            instance_id,
            device_name,
            version,
            features,
            group,
            mac,
        }
    }

    pub fn verify_mac(&self, psk: &str) -> bool {
        if let Some(ref received_mac) = self.mac {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            
            let mut hasher = DefaultHasher::new();
            self.instance_id.hash(&mut hasher);
            self.device_name.hash(&mut hasher);
            self.group.hash(&mut hasher);
            psk.hash(&mut hasher);
            
            let computed_mac = hasher.finish().to_le_bytes().to_vec();
            computed_mac == *received_mac
        } else {
            false
        }
    }
}

impl ClipboardUpdateMessage {
    pub fn new(content: String) -> Self {
        Self {
            content,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
}
