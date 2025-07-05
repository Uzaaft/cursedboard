mod config;
mod error;

use std::{net::TcpStream, sync::{Arc, Mutex, mpsc}, thread, time::Duration};

use objc2::rc::{Retained, autoreleasepool};
use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};
use objc2_foundation::NSString;
use shared::{ClipboardMessage, config::Config, network::{NetworkManager, handle_incoming_messages, send_clipboard_message}};

/// Wrapper around apple Pasteboard
struct Clipboard(Retained<NSPasteboard>);

impl Default for Clipboard {
    fn default() -> Self {
        let pasteboard = unsafe { NSPasteboard::generalPasteboard() };
        Clipboard(pasteboard)
    }
}

enum ClipboardCommand {
    SetText(String),
}

struct ClipboardState {
    changecount: isize,
    content: Option<String>,
}

/// Global connections map for broadcasting clipboard changes
type ConnectionsMap = Arc<Mutex<Vec<Arc<Mutex<TcpStream>>>>>;

fn clipboard_manager(clipboard_tx: mpsc::Receiver<ClipboardCommand>, state_tx: mpsc::Sender<ClipboardState>) {
    let clipboard = Clipboard::default();
    let mut prev_count = 0;
    
    loop {
        // Check for commands
        match clipboard_tx.try_recv() {
            Ok(ClipboardCommand::SetText(text)) => {
                autoreleasepool(|_pool| {
                    unsafe {
                        clipboard.0.clearContents();
                        let ns_string = NSString::from_str(&text);
                        clipboard.0.setString_forType(&ns_string, NSPasteboardTypeString);
                    }
                });
                // Update our count to avoid re-sending this change
                prev_count = unsafe { clipboard.0.changeCount() };
            }
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => break,
        }
        
        // Check for clipboard changes
        let changecount = unsafe { clipboard.0.changeCount() };
        
        if changecount != prev_count {
            let text = unsafe { clipboard.0.stringForType(NSPasteboardTypeString) };
            let content = text.map(|s| {
                autoreleasepool(|pool| {
                    unsafe { s.to_str(pool).to_string() }
                })
            });
            
            let state = ClipboardState { changecount, content };
            if let Err(e) = state_tx.send(state) {
                eprintln!("Failed to send clipboard state: {}", e);
                break;
            }
            
            prev_count = changecount;
        }
        
        thread::sleep(Duration::from_millis(100));
    }
}

fn monitor_clipboard_changes(state_rx: mpsc::Receiver<ClipboardState>, connections: ConnectionsMap) {
    let mut prev_count = 0;
    
    loop {
        match state_rx.recv_timeout(Duration::from_millis(500)) {
            Ok(state) => {
                if state.changecount != prev_count {
                    if let Some(content) = state.content {
                        let msg = ClipboardMessage::new(content);
                        
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
                    }
                    
                    prev_count = state.changecount;
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
}

fn handle_connection(stream: TcpStream, addr: std::net::SocketAddr, clipboard_tx: mpsc::Sender<ClipboardCommand>, connections: ConnectionsMap) {
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
        if let Err(e) = clipboard_tx.send(ClipboardCommand::SetText(msg.content)) {
            eprintln!("Failed to send clipboard command: {}", e);
            return Err(std::io::Error::new(std::io::ErrorKind::Other, e));
        }
        Ok(())
    });
    
    if let Err(e) = result {
        eprintln!("Connection error from {}: {}", addr, e);
    }
    
    // Remove from connections list
    connections.lock().unwrap().retain(|s| !Arc::ptr_eq(s, &send_stream));
}

fn main() -> Result<(), error::AppError> {
    // Load configuration
    let config = Config::load_layered()
        .or_else(|_| Config::from_env())
        .map_err(|e| error::AppError::ConfigError(e.to_string()))?;
    
    let bind_addr = config.bind_addr();
    let peer_addrs = config.peer_addrs();
    
    // Create network manager
    let network = NetworkManager::new(bind_addr, peer_addrs);
    
    // Create shared connections list
    let connections: ConnectionsMap = Arc::new(Mutex::new(Vec::new()));
    
    // Create clipboard communication channels
    let (clipboard_tx, clipboard_rx) = mpsc::channel();
    let (state_tx, state_rx) = mpsc::channel();
    
    // Spawn clipboard manager thread
    thread::spawn(move || {
        clipboard_manager(clipboard_rx, state_tx);
    });
    
    // Spawn clipboard monitor thread
    let connections_clone = connections.clone();
    thread::spawn(move || {
        monitor_clipboard_changes(state_rx, connections_clone);
    });
    
    // Start listening for incoming connections
    let clipboard_tx_clone = clipboard_tx.clone();
    let connections_clone = connections.clone();
    network.start_listener(move |stream, addr| {
        handle_connection(stream, addr, clipboard_tx_clone.clone(), connections_clone.clone());
    })?;
    
    // Start connecting to peers
    let clipboard_tx_clone = clipboard_tx.clone();
    let connections_clone = connections.clone();
    network.start_peer_connections(move |stream, addr| {
        handle_connection(stream, addr, clipboard_tx_clone.clone(), connections_clone.clone());
    });
    
    // Keep main thread alive
    loop {
        thread::sleep(Duration::from_secs(60));
        let conn_count = connections.lock().unwrap().len();
        println!("Active connections: {}", conn_count);
    }
}