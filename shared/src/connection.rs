use crate::discovery::DiscoveredPeer;
use crate::instance::Instance;
use crate::protocol::{encode_message, decode_message_length, HelloMessage, Message};
use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, RwLock};
use tokio::time;
use uuid::Uuid;

#[derive(Debug)]
pub struct ConnectionError(String);

impl std::fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ConnectionError {}

impl From<Box<dyn std::error::Error + Send + Sync>> for ConnectionError {
    fn from(e: Box<dyn std::error::Error + Send + Sync>) -> Self {
        ConnectionError(e.to_string())
    }
}

impl From<std::io::Error> for ConnectionError {
    fn from(e: std::io::Error) -> Self {
        ConnectionError(e.to_string())
    }
}

impl From<bincode::Error> for ConnectionError {
    fn from(e: bincode::Error) -> Self {
        ConnectionError(e.to_string())
    }
}

impl From<&str> for ConnectionError {
    fn from(s: &str) -> Self {
        ConnectionError(s.to_string())
    }
}

impl From<crate::instance::InstanceError> for ConnectionError {
    fn from(e: crate::instance::InstanceError) -> Self {
        ConnectionError(e.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PeerState {
    Discovered,
    Connecting,
    Connected,
}

struct PeerInfo {
    state: PeerState,
    address: SocketAddr,
    device_name: String,
    group: String,
}

#[derive(Clone)]
pub struct ConnectionManager {
    instance: Arc<RwLock<Instance>>,
    peers: Arc<RwLock<HashMap<Uuid, PeerInfo>>>,
    psk: Option<String>,
    pair_mode: Arc<RwLock<bool>>,
    inbound_clipboard_tx: mpsc::UnboundedSender<String>,
    inbound_clipboard_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<String>>>>,
}

impl ConnectionManager {
    pub fn new(
        instance: Instance,
        psk: Option<String>,
    ) -> Self {
        let (inbound_tx, inbound_rx) = mpsc::unbounded_channel();

        ConnectionManager {
            instance: Arc::new(RwLock::new(instance)),
            peers: Arc::new(RwLock::new(HashMap::new())),
            psk,
            pair_mode: Arc::new(RwLock::new(false)),
            inbound_clipboard_tx: inbound_tx,
            inbound_clipboard_rx: Arc::new(RwLock::new(Some(inbound_rx))),
        }
    }

    pub fn enable_pairing(&self, duration: Duration) {
        let pair_mode = self.pair_mode.clone();
        tokio::spawn(async move {
            *pair_mode.write().await = true;
            info!("Pairing mode enabled for {} seconds", duration.as_secs());
            time::sleep(duration).await;
            *pair_mode.write().await = false;
            info!("Pairing mode disabled");
        });
    }

    pub async fn handle_discovered_peer(&self, peer: DiscoveredPeer) {
        let instance = self.instance.read().await;
        
        if peer.instance_id == instance.id {
            debug!("Ignoring self-discovery");
            return;
        }

        if peer.group != instance.get_group() && !*self.pair_mode.read().await {
            debug!(
                "Ignoring peer {} - group mismatch (theirs: {}, ours: {})",
                peer.device_name,
                peer.group,
                instance.get_group()
            );
            return;
        }

        let is_allowed = instance.is_peer_allowed(&peer.instance_id);
        let pair_mode = *self.pair_mode.read().await;
        let allowlist_empty = instance.allowed_peers.is_empty();

        if !is_allowed && !pair_mode && !allowlist_empty {
            debug!(
                "Ignoring peer {} - not in allowlist and not in pairing mode",
                peer.device_name
            );
            return;
        }

        {
            let mut peers = self.peers.write().await;
            if let Some(info) = peers.get(&peer.instance_id) {
                if info.state == PeerState::Connected || info.state == PeerState::Connecting {
                    debug!("Already connected/connecting to peer {}", peer.device_name);
                    return;
                }
            }

            peers.insert(
                peer.instance_id,
                PeerInfo {
                    state: PeerState::Connecting,
                    address: peer.address,
                    device_name: peer.device_name.clone(),
                    group: peer.group.clone(),
                },
            );
        }
        drop(instance);

        let instance = self.instance.clone();
        let peers = self.peers.clone();
        let psk = self.psk.clone();
        let clipboard_tx = self.inbound_clipboard_tx.clone();

        tokio::spawn(async move {
            if let Err(e) = Self::connect_to_peer(
                peer.clone(),
                instance,
                peers.clone(),
                psk,
                clipboard_tx,
            )
            .await
            {
                error!("Failed to connect to peer {}: {}", peer.device_name, e);
                peers.write().await.remove(&peer.instance_id);
            }
        });
    }

    async fn connect_to_peer(
        peer: DiscoveredPeer,
        instance: Arc<RwLock<Instance>>,
        peers: Arc<RwLock<HashMap<Uuid, PeerInfo>>>,
        psk: Option<String>,
        clipboard_tx: mpsc::UnboundedSender<String>,
    ) -> Result<(), ConnectionError> {
        info!("Connecting to peer {} at {}", peer.device_name, peer.address);

        let mut stream = TcpStream::connect(peer.address).await?;

        let inst = instance.read().await;
        let hello = HelloMessage::new(
            inst.id,
            inst.device_name.clone(),
            inst.get_group(),
            psk.as_deref(),
        );
        drop(inst);

        let hello_msg = Message::Hello(hello);
        let encoded = encode_message(&hello_msg)?;
        stream.write_all(&encoded).await?;

        let mut len_buf = [0u8; 8];
        stream.read_exact(&mut len_buf).await?;
        let msg_len = decode_message_length(&len_buf) as usize;

        let mut msg_buf = vec![0u8; msg_len];
        stream.read_exact(&mut msg_buf).await?;

        let peer_hello = match Message::from_bytes(&msg_buf)? {
            Message::Hello(h) => h,
            _ => return Err("Expected HELLO message".into()),
        };

        if let Some(ref key) = psk {
            if peer_hello.mac.is_none() || !peer_hello.verify_mac(key) {
                return Err("PSK verification failed".into());
            }
        }

        if peer_hello.instance_id == instance.read().await.id {
            debug!("Detected self-connection, closing");
            return Ok(());
        }

        let should_close = if peer_hello.instance_id > instance.read().await.id {
            debug!("Deduplication: we have higher ID, keeping this connection");
            false
        } else {
            debug!("Deduplication: we have lower ID, may close if peer connects");
            false
        };

        if should_close {
            return Ok(());
        }

        {
            let mut inst = instance.write().await;
            if !inst.is_peer_allowed(&peer_hello.instance_id) {
                inst.add_peer(peer_hello.instance_id)?;
                info!("Added peer {} to allowlist", peer_hello.device_name);
            }
        }

        {
            let mut p = peers.write().await;
            if let Some(info) = p.get_mut(&peer_hello.instance_id) {
                info.state = PeerState::Connected;
            }
        }

        info!("Successfully connected to peer {}", peer_hello.device_name);

        Self::handle_connection(stream, clipboard_tx).await?;

        {
            let mut p = peers.write().await;
            p.remove(&peer_hello.instance_id);
        }

        Ok(())
    }

    pub async fn accept_connections(
        &self,
        listener: TcpListener,
    ) -> Result<(), ConnectionError> {
        let instance = self.instance.clone();
        let peers = self.peers.clone();
        let psk = self.psk.clone();
        let clipboard_tx = self.inbound_clipboard_tx.clone();

        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, addr)) => {
                        debug!("Accepted connection from {}", addr);
                        let instance = instance.clone();
                        let peers = peers.clone();
                        let psk = psk.clone();
                        let clipboard_tx = clipboard_tx.clone();

                        tokio::spawn(async move {
                            if let Err(e) = Self::handle_incoming_connection(
                                stream,
                                instance,
                                peers,
                                psk,
                                clipboard_tx,
                            )
                            .await
                            {
                                error!("Error handling incoming connection from {}: {}", addr, e);
                            }
                        });
                    }
                    Err(e) => {
                        error!("Error accepting connection: {}", e);
                    }
                }
            }
        });

        Ok(())
    }

    async fn handle_incoming_connection(
        mut stream: TcpStream,
        instance: Arc<RwLock<Instance>>,
        peers: Arc<RwLock<HashMap<Uuid, PeerInfo>>>,
        psk: Option<String>,
        clipboard_tx: mpsc::UnboundedSender<String>,
    ) -> Result<(), ConnectionError> {
        let mut len_buf = [0u8; 8];
        stream.read_exact(&mut len_buf).await?;
        let msg_len = decode_message_length(&len_buf) as usize;

        let mut msg_buf = vec![0u8; msg_len];
        stream.read_exact(&mut msg_buf).await?;

        let peer_hello = match Message::from_bytes(&msg_buf)? {
            Message::Hello(h) => h,
            _ => return Err("Expected HELLO message".into()),
        };

        if let Some(ref key) = psk {
            if peer_hello.mac.is_none() || !peer_hello.verify_mac(key) {
                return Err("PSK verification failed".into());
            }
        }

        if peer_hello.instance_id == instance.read().await.id {
            debug!("Detected self-connection, closing");
            return Ok(());
        }

        let inst = instance.read().await;
        let hello = HelloMessage::new(
            inst.id,
            inst.device_name.clone(),
            inst.get_group(),
            psk.as_deref(),
        );
        drop(inst);

        let hello_msg = Message::Hello(hello);
        let encoded = encode_message(&hello_msg)?;
        stream.write_all(&encoded).await?;

        {
            let mut inst = instance.write().await;
            if !inst.is_peer_allowed(&peer_hello.instance_id) {
                inst.add_peer(peer_hello.instance_id)?;
                info!("Added peer {} to allowlist", peer_hello.device_name);
            }
        }

        {
            let mut p = peers.write().await;
            p.insert(
                peer_hello.instance_id,
                PeerInfo {
                    state: PeerState::Connected,
                    address: stream.peer_addr()?,
                    device_name: peer_hello.device_name.clone(),
                    group: peer_hello.group.clone(),
                },
            );
        }

        info!("Accepted connection from peer {}", peer_hello.device_name);

        Self::handle_connection(stream, clipboard_tx).await?;

        {
            let mut p = peers.write().await;
            p.remove(&peer_hello.instance_id);
        }

        Ok(())
    }

    async fn handle_connection(
        mut stream: TcpStream,
        clipboard_tx: mpsc::UnboundedSender<String>,
    ) -> Result<(), ConnectionError> {
        loop {
            let mut len_buf = [0u8; 8];
            match stream.read_exact(&mut len_buf).await {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    debug!("Connection closed by peer");
                    break;
                }
                Err(e) => return Err(e.into()),
            }

            let msg_len = decode_message_length(&len_buf) as usize;
            let mut msg_buf = vec![0u8; msg_len];
            stream.read_exact(&mut msg_buf).await?;

            match Message::from_bytes(&msg_buf)? {
                Message::ClipboardUpdate(update) => {
                    debug!("Received clipboard update ({} bytes)", update.content.len());
                    let _ = clipboard_tx.send(update.content);
                }
                Message::Keepalive => {
                    debug!("Received keepalive");
                }
                Message::Hello(_) => {
                    warn!("Unexpected HELLO message during connection");
                }
            }
        }

        Ok(())
    }

    pub async fn next_clipboard_update(&self) -> Option<String> {
        let mut rx_lock = self.inbound_clipboard_rx.write().await;
        if let Some(rx) = rx_lock.as_mut() {
            rx.recv().await
        } else {
            None
        }
    }
}
