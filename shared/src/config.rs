use std::env;
use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct Config {
    pub remote_addr: SocketAddr,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let host = env::var("CURSEDBOARD_HOST").unwrap_or_else(|_| "172.16.104.129".to_string());
        let port = env::var("CURSEDBOARD_PORT")
            .unwrap_or_else(|_| "34254".to_string())
            .parse::<u16>()
            .map_err(|e| format!("Invalid port: {}", e))?;
        
        let addr_str = format!("{}:{}", host, port);
        let remote_addr = addr_str
            .parse::<SocketAddr>()
            .map_err(|e| format!("Invalid socket address '{}': {}", addr_str, e))?;
        
        Ok(Config { remote_addr })
    }
    
    pub fn server_bind_addr() -> SocketAddr {
        let port = env::var("CURSEDBOARD_PORT")
            .unwrap_or_else(|_| "34254".to_string())
            .parse::<u16>()
            .unwrap_or(34254);
        
        SocketAddr::from(([0, 0, 0, 0], port))
    }
}