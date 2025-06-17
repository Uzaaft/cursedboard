use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use arboard::Clipboard;
use shared::{ClipboardMessage, decode_message_length, encode_message, config::Config};

fn handle_incoming_messages(mut stream: TcpStream, clipboard: Arc<Mutex<Clipboard>>) {
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
                                
                                if let Ok(mut cb) = clipboard.lock() {
                                    if let Err(e) = cb.set_text(&msg.content) {
                                        eprintln!("Failed to set clipboard: {}", e);
                                    }
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

fn monitor_clipboard(stream: Arc<Mutex<TcpStream>>, clipboard: Arc<Mutex<Clipboard>>) {
    let mut last_content = String::new();
    
    loop {
        thread::sleep(Duration::from_millis(500));
        
        if let Ok(mut cb) = clipboard.lock() {
            match cb.get_text() {
                Ok(content) => {
                    if content != last_content && !content.is_empty() {
                        last_content = content.clone();
                        
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
                }
                Err(e) => eprintln!("Failed to get clipboard: {}", e),
            }
        }
    }
}

fn handle_client(stream: TcpStream, address: std::net::SocketAddr) {
    println!("Connection received from {}!", address);
    
    let clipboard = Arc::new(Mutex::new(Clipboard::new().expect("Failed to create clipboard")));
    let stream_arc = Arc::new(Mutex::new(stream.try_clone().expect("Failed to clone stream")));
    
    let clipboard_clone = clipboard.clone();
    let monitor_thread = thread::spawn(move || {
        monitor_clipboard(stream_arc, clipboard_clone);
    });
    
    handle_incoming_messages(stream, clipboard);
    
    println!("Client {} disconnected", address);
    monitor_thread.join().ok();
}

fn main() -> std::io::Result<()> {
    let bind_addr = Config::server_bind_addr();
    let listener = TcpListener::bind(bind_addr)?;
    let addr = listener.local_addr()?;
    
    println!("Listening on {}", addr);
    
    loop {
        match listener.accept() {
            Ok((stream, address)) => {
                thread::spawn(move || {
                    handle_client(stream, address);
                });
            }
            Err(e) => {
                eprintln!("Failed to accept connection: {}", e);
            }
        }
    }
}