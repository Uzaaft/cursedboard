mod clipboard_provider;

use std::{
    net::TcpStream,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use log::{debug, info};

use clipboard_provider::LinuxClipboardProvider;
use shared::{
    clipboard::{spawn_clipboard_manager, ClipboardEvent, ConnectionHandler},
    config::Config,
    network::NetworkManager,
};

fn main() -> std::io::Result<()> {
    // Initialize logger
    env_logger::init();

    // Load configuration
    let config = Config::load_layered()
        .or_else(|_| Ok(Config::default()))
        .map_err(|e: shared::config::ConfigError| {
            std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
        })?;

    let bind_addr = config.bind_addr();
    let peer_addrs = config.peer_addrs();

    // Create network manager
    let network = NetworkManager::new(bind_addr, peer_addrs);

    // Create shared connections list
    let connections: Arc<Mutex<Vec<Arc<Mutex<TcpStream>>>>> = Arc::new(Mutex::new(Vec::new()));

    // Create clipboard provider
    let provider = Box::new(
        LinuxClipboardProvider::new()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?,
    );

    // Spawn clipboard manager
    let (clipboard_tx, event_rx) = spawn_clipboard_manager(provider, connections.clone());

    // Create connection handler
    let conn_handler = Arc::new(ConnectionHandler::new(clipboard_tx, connections.clone()));

    // Spawn event monitor thread (for logging/debugging)
    thread::spawn(move || loop {
        match event_rx.recv() {
            Ok(ClipboardEvent::LocalChange(content)) => {
                info!("Local clipboard changed: {} bytes", content.len());
            }
            Ok(ClipboardEvent::RemoteChange(content)) => {
                info!("Remote clipboard update: {} bytes", content.len());
            }
            Ok(ClipboardEvent::Shutdown) => {
                info!("Clipboard manager shutting down");
                break;
            }
            Err(_) => break,
        }
    });

    // Start listening for incoming connections
    let handler_clone = conn_handler.clone();
    network.start_listener(move |stream, addr| {
        handler_clone.handle_connection(stream, addr);
    })?;

    // Start connecting to peers
    let handler_clone = conn_handler.clone();
    network.start_peer_connections(move |stream, addr| {
        handler_clone.handle_connection(stream, addr);
    });

    // Keep main thread alive with periodic status updates
    loop {
        thread::sleep(Duration::from_secs(60));
        let conn_count = connections.lock().unwrap().len();
        debug!("Active connections: {}", conn_count);
    }
}
