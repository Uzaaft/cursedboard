mod config;
mod error;

use std::{io::{Read, Write}, net::TcpStream, sync::{Arc, Mutex, mpsc}, thread, time::Duration};

use objc2::rc::{Retained, autoreleasepool};
use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};
use objc2_foundation::NSString;
use shared::{ClipboardMessage, decode_message_length, encode_message, config::Config};

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
    GetChange,
}

struct ClipboardState {
    changecount: isize,
    content: Option<String>,
}

fn handle_incoming_messages(mut stream: TcpStream, clipboard_tx: mpsc::Sender<ClipboardCommand>) {
    let mut message_length = [0u8; 8];
    
    loop {
        match stream.read_exact(&mut message_length) {
            Ok(_) => {
                let length = decode_message_length(&message_length) as usize;
                let mut buffer = vec![0u8; length];
                
                match stream.read_exact(&mut buffer) {
                    Ok(_) => {
                        match ClipboardMessage::from_bytes(&buffer) {
                            Ok(msg) => {
                                println!("Received clipboard content: {} bytes", msg.content.len());
                                if let Err(e) = clipboard_tx.send(ClipboardCommand::SetText(msg.content)) {
                                    eprintln!("Failed to send clipboard command: {}", e);
                                    break;
                                }
                            }
                            Err(e) => eprintln!("Failed to decode message: {}", e),
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to read message data: {}", e);
                        break;
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to read message length: {}", e);
                break;
            }
        }
    }
}

fn monitor_clipboard(stream: Arc<Mutex<TcpStream>>, clipboard_rx: mpsc::Receiver<ClipboardState>) {
    let mut prev_count = 0;
    
    loop {
        match clipboard_rx.recv_timeout(Duration::from_millis(500)) {
            Ok(state) => {
                if state.changecount != prev_count {
                    if let Some(content) = state.content {
                        let msg = ClipboardMessage::new(content);
                        
                        match encode_message(&msg) {
                            Ok(data) => {
                                if let Ok(mut stream) = stream.lock() {
                                    if let Err(e) = stream.write_all(&data) {
                                        eprintln!("Failed to send clipboard: {}", e);
                                    } else {
                                        println!("Sent clipboard content: {} bytes", msg.content.len());
                                    }
                                }
                            }
                            Err(e) => eprintln!("Failed to encode message: {}", e),
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
            }
            Ok(ClipboardCommand::GetChange) => {
                // This is handled below
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

fn main() -> Result<(), error::AppError> {
    let config = Config::from_env().map_err(|e| error::AppError::ConfigError(e))?;
    let addr = config.remote_addr;
    
    loop {
        println!("Attempting to connect to {}", addr);
        
        match TcpStream::connect(addr) {
            Ok(stream) => {
                println!("Connected to {}", addr);
                
                let stream_arc = Arc::new(Mutex::new(stream.try_clone()?));
                
                // Create channels for clipboard communication
                let (clipboard_tx, clipboard_rx) = mpsc::channel();
                let (state_tx, state_rx) = mpsc::channel();
                
                // Spawn clipboard manager thread (handles all clipboard operations)
                let clipboard_thread = thread::spawn(move || {
                    clipboard_manager(clipboard_rx, state_tx);
                });
                
                // Spawn monitor thread
                let monitor_thread = thread::spawn(move || {
                    monitor_clipboard(stream_arc, state_rx);
                });
                
                // Handle incoming messages
                handle_incoming_messages(stream, clipboard_tx);
                
                println!("Disconnected from server");
                monitor_thread.join().ok();
                clipboard_thread.join().ok();
            }
            Err(e) => {
                eprintln!("Failed to connect: {}", e);
            }
        }
        
        thread::sleep(Duration::from_secs(5));
        println!("Reconnecting...");
    }
}