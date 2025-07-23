use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use log::{debug, error, info};

use crate::{decode_message_length, encode_message, ClipboardMessage};

/// Connection information
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub addr: SocketAddr,
    pub is_incoming: bool,
}

/// Manages network connections for peer-to-peer operation
pub struct NetworkManager {
    bind_addr: SocketAddr,
    peers: Vec<SocketAddr>,
    connections: Arc<Mutex<HashMap<SocketAddr, ConnectionInfo>>>,
    reconnect_delay: Duration,
}

impl NetworkManager {
    pub fn new(bind_addr: SocketAddr, peers: Vec<SocketAddr>) -> Self {
        Self {
            bind_addr,
            peers,
            connections: Arc::new(Mutex::new(HashMap::new())),
            reconnect_delay: Duration::from_secs(5),
        }
    }

    /// Start listening for incoming connections
    pub fn start_listener<F>(&self, handler: F) -> std::io::Result<()>
    where
        F: Fn(TcpStream, SocketAddr) + Send + Sync + 'static + Clone,
    {
        let listener = TcpListener::bind(self.bind_addr)?;
        let connections = self.connections.clone();

        info!("Listening on {}", listener.local_addr()?);

        thread::spawn(move || {
            loop {
                match listener.accept() {
                    Ok((stream, addr)) => {
                        // Check if we already have a connection to this peer
                        let should_accept = {
                            let conns = connections.lock().unwrap();
                            !conns.contains_key(&addr)
                        };

                        if should_accept {
                            info!("Accepted connection from {addr}");
                            connections.lock().unwrap().insert(
                                addr,
                                ConnectionInfo {
                                    addr,
                                    is_incoming: true,
                                },
                            );

                            let handler = handler.clone();
                            let connections = connections.clone();
                            thread::spawn(move || {
                                handler(stream, addr);
                                // Remove connection when done
                                connections.lock().unwrap().remove(&addr);
                                info!("Connection from {addr} closed");
                            });
                        } else {
                            debug!("Rejecting duplicate connection from {addr}");
                        }
                    }
                    Err(e) => error!("Failed to accept connection: {e}"),
                }
            }
        });

        Ok(())
    }

    /// Start connecting to configured peers
    pub fn start_peer_connections<F>(&self, handler: F)
    where
        F: Fn(TcpStream, SocketAddr) + Send + Sync + 'static + Clone,
    {
        for peer_addr in &self.peers {
            let peer_addr = *peer_addr;
            let connections = self.connections.clone();
            let handler = handler.clone();
            let reconnect_delay = self.reconnect_delay;

            thread::spawn(move || {
                loop {
                    // Check if we already have a connection
                    let should_connect = {
                        let conns = connections.lock().unwrap();
                        !conns.contains_key(&peer_addr)
                    };

                    if should_connect {
                        debug!("Attempting to connect to {peer_addr}");
                        match TcpStream::connect(peer_addr) {
                            Ok(stream) => {
                                info!("Connected to {peer_addr}");
                                connections.lock().unwrap().insert(
                                    peer_addr,
                                    ConnectionInfo {
                                        addr: peer_addr,
                                        is_incoming: false,
                                    },
                                );

                                handler(stream, peer_addr);

                                // Remove connection when done
                                connections.lock().unwrap().remove(&peer_addr);
                                info!("Disconnected from {peer_addr}");
                            }
                            Err(e) => {
                                error!("Failed to connect to {peer_addr}: {e}");
                            }
                        }
                    }

                    thread::sleep(reconnect_delay);
                }
            });
        }
    }

    /// Get current connections
    pub fn get_connections(&self) -> Vec<ConnectionInfo> {
        self.connections.lock().unwrap().values().cloned().collect()
    }
}

/// Handle incoming messages from a stream
pub fn handle_incoming_messages<F>(
    mut stream: TcpStream,
    mut message_handler: F,
) -> std::io::Result<()>
where
    F: FnMut(ClipboardMessage) -> std::io::Result<()>,
{
    let mut message_length = [0u8; 8];

    loop {
        match stream.read_exact(&mut message_length) {
            Ok(_) => {
                let length = decode_message_length(&message_length) as usize;
                let mut buffer = vec![0u8; length];

                match stream.read_exact(&mut buffer) {
                    Ok(_) => match ClipboardMessage::from_bytes(&buffer) {
                        Ok(msg) => {
                            message_handler(msg)?;
                        }
                        Err(e) => error!("Failed to decode message: {e}"),
                    },
                    Err(e) => {
                        error!("Failed to read message: {e}");
                        return Err(e);
                    }
                }
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
}

/// Send a clipboard message to a stream
pub fn send_clipboard_message(
    stream: &mut TcpStream,
    msg: &ClipboardMessage,
) -> std::io::Result<()> {
    let encoded = encode_message(msg).map_err(std::io::Error::other)?;
    stream.write_all(&encoded)?;
    stream.flush()?;
    Ok(())
}
