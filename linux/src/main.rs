use std::{net::TcpStream, sync::{Arc, Mutex}, thread, time::Duration};

use arboard::Clipboard;
use shared::{ClipboardMessage, config::Config, network::{NetworkManager, handle_incoming_messages, send_clipboard_message}};

/// Global connections map for broadcasting clipboard changes
type ConnectionsMap = Arc<Mutex<Vec<Arc<Mutex<TcpStream>>>>>;

fn monitor_clipboard_changes(connections: ConnectionsMap) {
    let mut clipboard = Clipboard::new().expect("Failed to initialize clipboard");
    let mut last_content = String::new();
    
    loop {
        if let Ok(current) = clipboard.get_text() {
            if current != last_content && !current.is_empty() {
                let msg = ClipboardMessage::new(current.clone());
                
                // Broadcast to all connected peers
                let conns = connections.lock().unwrap();
                for stream in conns.iter() {
                    if let Ok(mut stream) = stream.lock() {
                        if let Err(e) = send_clipboard_message(&mut *stream, &msg) {
                            eprintln!("Failed to send clipboard: {}", e);
                        } else {
                            println!("Sent clipboard content: {} bytes", msg.content.len());
                        }
                    }
                }
                
                last_content = current;
            }
        }
        
        thread::sleep(Duration::from_millis(500));
    }
}

fn handle_connection(stream: TcpStream, addr: std::net::SocketAddr, clipboard: Arc<Mutex<Clipboard>>, connections: ConnectionsMap) {
    // Clone the stream for sending
    let send_stream = match stream.try_clone() {
        Ok(s) => Arc::new(Mutex::new(s)),
        Err(e) => {
            eprintln!("Failed to clone stream: {}", e);
            return;
        }
    };
    
    // Add to connections list
    connections.lock().unwrap().push(send_stream.clone());
    
    // Handle incoming messages
    let result = handle_incoming_messages(stream, |msg| {
        println!("Received clipboard content from {}: {} bytes", addr, msg.content.len());
        
        if let Ok(mut clipboard) = clipboard.lock() {
            if let Err(e) = clipboard.set_text(msg.content) {
                eprintln!("Failed to set clipboard: {}", e);
                return Err(std::io::Error::new(std::io::ErrorKind::Other, e));
            }
        }
        Ok(())
    });
    
    if let Err(e) = result {
        eprintln!("Connection error from {}: {}", addr, e);
    }
    
    // Remove from connections list
    connections.lock().unwrap().retain(|s| !Arc::ptr_eq(s, &send_stream));
}

fn main() -> std::io::Result<()> {
    // Load configuration
    let config = Config::load_layered()
        .or_else(|_| Ok(Config::default()))
        .map_err(|e: shared::config::ConfigError| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    
    let bind_addr = config.bind_addr();
    let peer_addrs = config.peer_addrs();
    
    // Create network manager
    let network = NetworkManager::new(bind_addr, peer_addrs);
    
    // Create shared clipboard
    let clipboard = Arc::new(Mutex::new(
        Clipboard::new().expect("Failed to initialize clipboard")
    ));
    
    // Create shared connections list
    let connections: ConnectionsMap = Arc::new(Mutex::new(Vec::new()));
    
    // Spawn clipboard monitor thread
    let connections_clone = connections.clone();
    thread::spawn(move || {
        monitor_clipboard_changes(connections_clone);
    });
    
    // Start listening for incoming connections
    let clipboard_clone = clipboard.clone();
    let connections_clone = connections.clone();
    network.start_listener(move |stream, addr| {
        handle_connection(stream, addr, clipboard_clone.clone(), connections_clone.clone());
    })?;
    
    // Start connecting to peers
    let clipboard_clone = clipboard.clone();
    let connections_clone = connections.clone();
    network.start_peer_connections(move |stream, addr| {
        handle_connection(stream, addr, clipboard_clone.clone(), connections_clone.clone());
    });
    
    // Keep main thread alive
    loop {
        thread::sleep(Duration::from_secs(60));
        let conn_count = connections.lock().unwrap().len();
        println!("Active connections: {}", conn_count);
    }
}