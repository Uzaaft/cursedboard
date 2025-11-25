use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum TrustError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse error: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("serialize error: {0}")]
    Serialize(#[from] toml::ser::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustedPeer {
    pub name: String,
    pub first_seen: u64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TrustStore {
    peers: HashMap<Uuid, TrustedPeer>,
}

impl TrustStore {
    pub fn load() -> Result<Self, TrustError> {
        let path = Self::path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)?;
        Ok(toml::from_str(&content)?)
    }

    pub fn save(&self) -> Result<(), TrustError> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    pub fn is_trusted(&self, id: &Uuid) -> bool {
        self.peers.contains_key(id)
    }

    pub fn trust(&mut self, id: Uuid, name: String) {
        if !self.peers.contains_key(&id) {
            let peer = TrustedPeer {
                name,
                first_seen: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            };
            self.peers.insert(id, peer);
        }
    }

    pub fn get(&self, id: &Uuid) -> Option<&TrustedPeer> {
        self.peers.get(id)
    }

    fn path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("cursedboard")
            .join("trusted.toml")
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Instance {
    pub id: Uuid,
}

impl Instance {
    pub fn load_or_create() -> Result<Self, TrustError> {
        let path = Self::path();
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            return Ok(toml::from_str(&content)?);
        }
        let instance = Self { id: Uuid::new_v4() };
        instance.save()?;
        Ok(instance)
    }

    fn save(&self) -> Result<(), TrustError> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    fn path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("cursedboard")
            .join("instance.toml")
    }
}
