use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instance {
    pub id: Uuid,
    pub device_name: String,
    #[serde(default)]
    pub allowed_peers: Vec<Uuid>,
    pub group: Option<String>,
}

impl Instance {
    pub fn load_or_create() -> Result<Self, InstanceError> {
        let path = Self::default_path();
        
        if path.exists() {
            Self::load(&path)
        } else {
            let instance = Self::create_new()?;
            instance.save(&path)?;
            Ok(instance)
        }
    }

    pub fn create_new() -> Result<Self, InstanceError> {
        let id = Uuid::new_v4();
        let device_name = hostname::get()
            .ok()
            .and_then(|h| h.into_string().ok())
            .unwrap_or_else(|| "cursedboard".to_string());

        Ok(Instance {
            id,
            device_name,
            allowed_peers: Vec::new(),
            group: None,
        })
    }

    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, InstanceError> {
        let contents = fs::read_to_string(path)?;
        let instance: Instance = toml::from_str(&contents)?;
        Ok(instance)
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), InstanceError> {
        let contents = toml::to_string_pretty(self)?;
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, contents)?;
        Ok(())
    }

    pub fn default_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("cursedboard")
            .join("instance.toml")
    }

    pub fn add_peer(&mut self, peer_id: Uuid) -> Result<(), InstanceError> {
        if !self.allowed_peers.contains(&peer_id) {
            self.allowed_peers.push(peer_id);
            self.save(&Self::default_path())?;
        }
        Ok(())
    }

    pub fn is_peer_allowed(&self, peer_id: &Uuid) -> bool {
        self.allowed_peers.is_empty() || self.allowed_peers.contains(peer_id)
    }

    pub fn default_group() -> String {
        std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "default".to_string())
            .to_lowercase()
    }

    pub fn get_group(&self) -> String {
        self.group.clone().unwrap_or_else(Self::default_group)
    }
}

#[derive(Debug)]
pub enum InstanceError {
    Io(std::io::Error),
    TomlParse(toml::de::Error),
    TomlSerialize(toml::ser::Error),
}

impl std::fmt::Display for InstanceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InstanceError::Io(e) => write!(f, "IO error: {e}"),
            InstanceError::TomlParse(e) => write!(f, "TOML parsing error: {e}"),
            InstanceError::TomlSerialize(e) => write!(f, "TOML serialization error: {e}"),
        }
    }
}

impl std::error::Error for InstanceError {}

impl From<std::io::Error> for InstanceError {
    fn from(e: std::io::Error) -> Self {
        InstanceError::Io(e)
    }
}

impl From<toml::de::Error> for InstanceError {
    fn from(e: toml::de::Error) -> Self {
        InstanceError::TomlParse(e)
    }
}

impl From<toml::ser::Error> for InstanceError {
    fn from(e: toml::ser::Error) -> Self {
        InstanceError::TomlSerialize(e)
    }
}
