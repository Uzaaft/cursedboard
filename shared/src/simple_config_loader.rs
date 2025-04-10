use crate::config::{Config, ConfigError};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Simple configuration loader without external dependencies
pub struct SimpleConfigLoader {
    config_paths: Vec<PathBuf>,
}

impl SimpleConfigLoader {
    pub fn new() -> Self {
        Self {
            config_paths: Vec::new(),
        }
    }

    /// Add a configuration file path
    pub fn add_config_path<P: AsRef<Path>>(mut self, path: P) -> Self {
        let path = path.as_ref().to_path_buf();
        if path.exists() {
            self.config_paths.push(path);
        }
        self
    }

    /// Add default configuration paths for the platform
    pub fn add_default_paths(mut self) -> Self {
        // System-wide configuration
        #[cfg(unix)]
        {
            self = self.add_config_path("/etc/cursedboard/config.toml");
        }

        // User configuration
        self = self.add_config_path(Config::default_config_path());

        // Current directory configuration
        self = self.add_config_path("cursedboard.toml");
        self = self.add_config_path(".cursedboard.toml");

        self
    }

    /// Load and merge configurations
    pub fn load(self) -> Result<Config, ConfigError> {
        // Start with default config
        let mut config = Config::default();

        // Load and merge each config file
        for path in self.config_paths {
            if let Ok(contents) = fs::read_to_string(&path) {
                if let Ok(file_config) = toml::from_str::<toml::Value>(&contents) {
                    merge_config(&mut config, file_config)?;
                }
            }
        }

        // Apply environment variable overrides
        apply_env_overrides(&mut config)?;

        // Validate the final configuration
        config.validate()?;

        Ok(config)
    }
}

/// Merge a TOML value into the config
fn merge_config(config: &mut Config, toml_value: toml::Value) -> Result<(), ConfigError> {
    // Convert the current config to TOML value
    let mut current_value = toml::Value::try_from(&*config).map_err(ConfigError::TomlSerialize)?;

    // Merge the values
    merge_toml_values(&mut current_value, toml_value);

    // Convert back to Config
    *config = current_value.try_into().map_err(ConfigError::TomlParse)?;

    Ok(())
}

/// Recursively merge TOML values
fn merge_toml_values(base: &mut toml::Value, other: toml::Value) {
    match (base, other) {
        (toml::Value::Table(base_table), toml::Value::Table(other_table)) => {
            for (key, value) in other_table {
                match base_table.get_mut(&key) {
                    Some(base_value) => merge_toml_values(base_value, value),
                    None => {
                        base_table.insert(key, value);
                    }
                }
            }
        }
        (base, other) => *base = other,
    }
}

/// Apply environment variable overrides
fn apply_env_overrides(config: &mut Config) -> Result<(), ConfigError> {
    // CURSEDBOARD_HOST maps to a peer for backward compatibility
    if let Ok(host) = env::var("CURSEDBOARD_HOST") {
        if !host.is_empty() {
            let peer = crate::config::PeerConfig {
                host: host.parse()?,
                port: None,
                name: None,
            };
            // Only add if not already in peers list
            if !config.network.peers.iter().any(|p| p.host == peer.host) {
                config.network.peers.push(peer);
            }
        }
    }

    // CURSEDBOARD_PORT maps to network.port
    if let Ok(port) = env::var("CURSEDBOARD_PORT") {
        if !port.is_empty() {
            config.network.port = port.parse()?;
        }
    }

    // Mode is no longer used - both sides are symmetric

    // CURSEDBOARD_LOG_LEVEL maps to logging.level
    if let Ok(level) = env::var("CURSEDBOARD_LOG_LEVEL") {
        config.logging.level = level;
    }

    Ok(())
}

impl Default for SimpleConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}
