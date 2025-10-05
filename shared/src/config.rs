use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Main configuration structure for cursedboard
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct Config {
    /// Network configuration
    pub network: NetworkConfig,

    /// Clipboard synchronization settings
    pub clipboard: ClipboardConfig,

    /// Logging configuration
    pub logging: LoggingConfig,

    /// Platform-specific settings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<PlatformConfig>,

    /// Peer-to-peer configuration (for future use)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub p2p: Option<P2PConfig>,
}

/// Network configuration for symmetric peer operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Address to bind for incoming connections (0.0.0.0 for all interfaces)
    pub bind_addr: IpAddr,

    /// Port to listen on
    pub port: u16,

    /// List of peer addresses to connect to
    #[serde(default)]
    pub peers: Vec<PeerConfig>,

    /// Connection timeout
    #[serde(with = "crate::duration_serde")]
    pub connection_timeout: Duration,

    /// Reconnection settings
    pub reconnect: ReconnectConfig,

    /// TLS/SSL configuration (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls: Option<TlsConfig>,
}

/// Peer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerConfig {
    /// Peer address
    pub host: IpAddr,

    /// Peer port (defaults to same as local port)
    pub port: Option<u16>,

    /// Optional peer name/identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Reconnection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconnectConfig {
    /// Enable automatic reconnection
    pub enabled: bool,

    /// Initial delay before reconnecting
    #[serde(with = "crate::duration_serde")]
    pub initial_delay: Duration,

    /// Maximum delay between reconnection attempts
    #[serde(with = "crate::duration_serde")]
    pub max_delay: Duration,

    /// Maximum number of reconnection attempts (None = infinite)
    pub max_attempts: Option<u32>,
}

/// TLS/SSL configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Enable TLS
    pub enabled: bool,

    /// Path to certificate file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cert_path: Option<PathBuf>,

    /// Path to private key file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_path: Option<PathBuf>,

    /// Path to CA certificate (for client verification)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ca_path: Option<PathBuf>,

    /// Skip certificate verification (INSECURE - only for testing)
    pub skip_verify: bool,
}

/// Clipboard synchronization settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardConfig {
    /// Check interval for clipboard changes
    #[serde(with = "crate::duration_serde")]
    pub check_interval: Duration,

    /// Maximum size of clipboard content to sync (in bytes)
    pub max_size: usize,

    /// Sync only text content
    pub text_only: bool,

    /// Ignore empty clipboard
    pub ignore_empty: bool,

    /// Patterns to filter out (regex)
    #[serde(default)]
    pub ignore_patterns: Vec<String>,

    /// Enable bidirectional sync
    pub bidirectional: bool,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: String,

    /// Log to file
    pub file: Option<PathBuf>,

    /// Log format (json, pretty, compact)
    pub format: LogFormat,

    /// Maximum log file size (for rotation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_size: Option<String>,

    /// Number of log files to keep
    pub max_files: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    Json,
    Pretty,
    Compact,
}

/// Platform-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum PlatformConfig {
    MacOS(MacOSConfig),
    Linux(LinuxConfig),
}

/// macOS-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacOSConfig {
    /// Use Universal Clipboard (Handoff)
    pub use_universal_clipboard: bool,

    /// Monitor specific pasteboard types
    #[serde(default)]
    pub pasteboard_types: Vec<String>,
}

/// Linux-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinuxConfig {
    /// Clipboard selection to monitor (clipboard, primary, secondary)
    pub selection: ClipboardSelection,

    /// X11 display (if not default)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,

    /// Wayland-specific settings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wayland: Option<WaylandConfig>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClipboardSelection {
    Clipboard,
    Primary,
    Secondary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaylandConfig {
    /// Force Wayland backend
    pub force_wayland: bool,
}

/// Peer-to-peer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2PConfig {
    /// Discovery method
    pub discovery: DiscoveryConfig,

    /// Encryption settings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encryption: Option<EncryptionConfig>,

    /// Device name/identifier
    pub device_name: String,

    /// Allowed peers (whitelist) - managed automatically
    #[serde(default)]
    pub allowed_peers: Vec<String>,

    /// Blocked peers (blacklist)
    #[serde(default)]
    pub blocked_peers: Vec<String>,

    /// Group name for peer filtering (defaults to username)
    pub group: Option<String>,

    /// Pairing mode timeout (in seconds) - accept first new peer
    pub pair_timeout: Option<u64>,

    /// Pre-shared key for authentication
    pub psk: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    /// Enable mDNS/Bonjour discovery
    pub mdns: bool,

    /// Enable manual peer configuration
    pub manual: bool,

    /// List of manual peer addresses
    #[serde(default)]
    pub peers: Vec<SocketAddr>,

    /// Discovery timeout
    #[serde(with = "crate::duration_serde")]
    pub timeout: Duration,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            mdns: true,
            manual: true,
            peers: Vec::new(),
            timeout: Duration::from_secs(2),
        }
    }
}

impl Default for P2PConfig {
    fn default() -> Self {
        let device_name = hostname::get()
            .ok()
            .and_then(|h| h.into_string().ok())
            .unwrap_or_else(|| "cursedboard".to_string());

        Self {
            discovery: DiscoveryConfig::default(),
            encryption: None,
            device_name,
            allowed_peers: Vec::new(),
            blocked_peers: Vec::new(),
            group: None,
            pair_timeout: None,
            psk: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    /// Enable end-to-end encryption
    pub enabled: bool,

    /// Encryption method
    pub method: String,

    /// Path to public key
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_key_path: Option<PathBuf>,

    /// Path to private key
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_key_path: Option<PathBuf>,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            bind_addr: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), // Bind to all interfaces
            port: 34254,
            peers: Vec::new(), // No default peers
            connection_timeout: Duration::from_secs(10),
            reconnect: ReconnectConfig::default(),
            tls: None,
        }
    }
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            max_attempts: None,
        }
    }
}

impl Default for ClipboardConfig {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_millis(500),
            max_size: 10 * 1024 * 1024, // 10MB
            text_only: true,
            ignore_empty: true,
            ignore_patterns: vec![],
            bidirectional: true,
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            file: None,
            format: LogFormat::Pretty,
            max_size: Some("10MB".to_string()),
            max_files: 5,
        }
    }
}

/// Configuration file locations
impl Config {
    /// Get the default config file path for the current platform
    pub fn default_config_path() -> PathBuf {
        #[cfg(target_os = "macos")]
        {
            dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from("~/.config"))
                .join("cursedboard")
                .join("config.toml")
        }

        #[cfg(target_os = "linux")]
        {
            // Follow XDG Base Directory specification
            env::var("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| {
                    dirs::config_dir().unwrap_or_else(|| PathBuf::from("~/.config"))
                })
                .join("cursedboard")
                .join("config.toml")
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            PathBuf::from("cursedboard.toml")
        }
    }

    /// Load configuration from file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let contents = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&contents)?;
        config.validate()?;
        Ok(config)
    }

    /// Load configuration from default location
    pub fn load_default() -> Result<Self, ConfigError> {
        let path = Self::default_config_path();
        if path.exists() {
            Self::load(path)
        } else {
            Ok(Self::default())
        }
    }

    /// Load configuration with environment variable overrides
    pub fn load_with_env() -> Result<Self, ConfigError> {
        let mut config = Self::load_default()?;

        // Override with environment variables
        if let Ok(host) = env::var("CURSEDBOARD_HOST") {
            // For backward compat, CURSEDBOARD_HOST becomes a peer
            let peer = PeerConfig {
                host: host.parse()?,
                port: None,
                name: None,
            };
            config.network.peers.push(peer);
        }

        if let Ok(port) = env::var("CURSEDBOARD_PORT") {
            config.network.port = port.parse()?;
        }

        // Mode is no longer used - both sides are symmetric

        if let Ok(level) = env::var("CURSEDBOARD_LOG_LEVEL") {
            config.logging.level = level;
        }

        Ok(config)
    }

    /// Load configuration using the SimpleConfigLoader with all sources
    pub fn load_layered() -> Result<Self, ConfigError> {
        use crate::simple_config_loader::SimpleConfigLoader;

        SimpleConfigLoader::new().add_default_paths().load()
    }

    /// Create a simple config from environment (backward compatibility)
    pub fn from_env() -> Result<Self, ConfigError> {
        let mut config = Self::default();

        // Apply environment variables
        if let Ok(host) = env::var("CURSEDBOARD_HOST") {
            // For backward compat, CURSEDBOARD_HOST becomes a peer
            let peer = PeerConfig {
                host: host.parse()?,
                port: None,
                name: None,
            };
            config.network.peers.push(peer);
        } else {
            // Use the legacy default as peer
            let peer = PeerConfig {
                host: "172.16.104.129".parse()?,
                port: None,
                name: None,
            };
            config.network.peers.push(peer);
        }

        if let Ok(port) = env::var("CURSEDBOARD_PORT") {
            config.network.port = port.parse()?;
        }

        Ok(config)
    }

    /// Get server bind address (backward compatibility)
    pub fn server_bind_addr() -> SocketAddr {
        let port = env::var("CURSEDBOARD_PORT")
            .unwrap_or_else(|_| "34254".to_string())
            .parse::<u16>()
            .unwrap_or(34254);

        SocketAddr::from(([0, 0, 0, 0], port))
    }

    /// Save configuration to file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), ConfigError> {
        let contents = toml::to_string_pretty(self)?;
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, contents)?;
        Ok(())
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate port range
        if self.network.port == 0 {
            return Err(ConfigError::InvalidPort(0));
        }

        // Validate log level
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.logging.level.to_lowercase().as_str()) {
            return Err(ConfigError::InvalidLogLevel(self.logging.level.clone()));
        }

        // Validate clipboard settings
        if self.clipboard.max_size == 0 {
            return Err(ConfigError::InvalidClipboardSize(0));
        }

        // Validate regex patterns
        for pattern in &self.clipboard.ignore_patterns {
            regex::Regex::new(pattern)
                .map_err(|e| ConfigError::InvalidRegex(pattern.clone(), e))?;
        }

        Ok(())
    }

    /// Get bind address for listening
    pub fn bind_addr(&self) -> SocketAddr {
        SocketAddr::new(self.network.bind_addr, self.network.port)
    }

    /// Get peer addresses for connecting
    pub fn peer_addrs(&self) -> Vec<SocketAddr> {
        self.network
            .peers
            .iter()
            .map(|peer| {
                let port = peer.port.unwrap_or(self.network.port);
                SocketAddr::new(peer.host, port)
            })
            .collect()
    }

    /// Get socket address (backward compatibility - returns bind address)
    pub fn socket_addr(&self) -> SocketAddr {
        self.bind_addr()
    }
}

/// Configuration errors
#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    TomlParse(toml::de::Error),
    TomlSerialize(toml::ser::Error),
    InvalidIpAddr(std::net::AddrParseError),
    InvalidPort(u16),
    InvalidLogLevel(String),
    InvalidClipboardSize(usize),
    InvalidRegex(String, regex::Error),
    InvalidInt(std::num::ParseIntError),
    ConfigBuild(String),
    ConfigDeserialize(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Io(e) => write!(f, "IO error: {e}"),
            ConfigError::TomlParse(e) => write!(f, "TOML parsing error: {e}"),
            ConfigError::TomlSerialize(e) => write!(f, "TOML serialization error: {e}"),
            ConfigError::InvalidIpAddr(e) => write!(f, "Invalid IP address: {e}"),
            ConfigError::InvalidPort(port) => write!(f, "Invalid port number: {port}"),
            ConfigError::InvalidLogLevel(level) => write!(f, "Invalid log level: {level}"),
            ConfigError::InvalidClipboardSize(size) => write!(f, "Invalid clipboard size: {size}"),
            ConfigError::InvalidRegex(pattern, e) => {
                write!(f, "Invalid regex pattern '{pattern}': {e}")
            }
            ConfigError::InvalidInt(e) => write!(f, "Invalid integer: {e}"),
            ConfigError::ConfigBuild(msg) => write!(f, "Configuration build error: {msg}"),
            ConfigError::ConfigDeserialize(msg) => {
                write!(f, "Configuration deserialization error: {msg}")
            }
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConfigError::Io(e) => Some(e),
            ConfigError::TomlParse(e) => Some(e),
            ConfigError::TomlSerialize(e) => Some(e),
            ConfigError::InvalidIpAddr(e) => Some(e),
            ConfigError::InvalidRegex(_, e) => Some(e),
            ConfigError::InvalidInt(e) => Some(e),
            _ => None,
        }
    }
}

// Implement From conversions for automatic error conversion
impl From<std::io::Error> for ConfigError {
    fn from(e: std::io::Error) -> Self {
        ConfigError::Io(e)
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(e: toml::de::Error) -> Self {
        ConfigError::TomlParse(e)
    }
}

impl From<toml::ser::Error> for ConfigError {
    fn from(e: toml::ser::Error) -> Self {
        ConfigError::TomlSerialize(e)
    }
}

impl From<std::net::AddrParseError> for ConfigError {
    fn from(e: std::net::AddrParseError) -> Self {
        ConfigError::InvalidIpAddr(e)
    }
}

impl From<std::num::ParseIntError> for ConfigError {
    fn from(e: std::num::ParseIntError) -> Self {
        ConfigError::InvalidInt(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.network.port, 34254);
        assert_eq!(
            config.network.bind_addr,
            IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0))
        );
        assert_eq!(config.clipboard.check_interval, Duration::from_millis(500));
    }

    #[test]
    fn test_config_validation() {
        let mut config = Config::default();
        config.network.port = 0;
        assert!(config.validate().is_err());

        config.network.port = 8080;
        config.logging.level = "invalid".to_string();
        assert!(config.validate().is_err());

        config.logging.level = "info".to_string();
        assert!(config.validate().is_ok());
    }
}
