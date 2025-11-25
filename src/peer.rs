use crate::protocol::{
    compute_auth_response, generate_challenge, verify_auth_response, Message, ProtocolError,
};
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tracing::{debug, info};
use uuid::Uuid;

#[derive(Debug)]
pub enum PeerEvent {
    Connected { id: Uuid, name: String },
    Clipboard { content: String, timestamp: u64 },
    Disconnected { id: Uuid },
}

pub struct PeerConnection {
    stream: TcpStream,
    peer_id: Option<Uuid>,
    peer_name: Option<String>,
}

impl PeerConnection {
    pub async fn connect(addr: SocketAddr) -> Result<Self, ProtocolError> {
        let stream = TcpStream::connect(addr).await?;
        Ok(Self {
            stream,
            peer_id: None,
            peer_name: None,
        })
    }

    pub fn from_stream(stream: TcpStream) -> Self {
        Self {
            stream,
            peer_id: None,
            peer_name: None,
        }
    }

    pub async fn handshake_outbound(
        &mut self,
        our_id: Uuid,
        our_name: &str,
        psk: &str,
    ) -> Result<(Uuid, String), ProtocolError> {
        let hello = Message::Hello {
            id: our_id,
            name: our_name.to_string(),
        };
        self.send(&hello).await?;

        let their_hello = self.recv().await?;
        let (their_id, their_name) = match their_hello {
            Message::Hello { id, name } => (id, name),
            _ => return Err(ProtocolError::AuthFailed),
        };

        let challenge = generate_challenge();
        let auth = Message::Auth {
            challenge,
            response: [0u8; 32],
        };
        self.send(&auth).await?;

        let their_auth = self.recv().await?;
        match their_auth {
            Message::Auth { response, .. } => {
                if !verify_auth_response(psk, &challenge, &response) {
                    return Err(ProtocolError::AuthFailed);
                }
            }
            _ => return Err(ProtocolError::AuthFailed),
        }

        self.peer_id = Some(their_id);
        self.peer_name = Some(their_name.clone());
        Ok((their_id, their_name))
    }

    pub async fn handshake_inbound(
        &mut self,
        our_id: Uuid,
        our_name: &str,
        psk: &str,
    ) -> Result<(Uuid, String), ProtocolError> {
        let their_hello = self.recv().await?;
        let (their_id, their_name) = match their_hello {
            Message::Hello { id, name } => (id, name),
            _ => return Err(ProtocolError::AuthFailed),
        };

        let hello = Message::Hello {
            id: our_id,
            name: our_name.to_string(),
        };
        self.send(&hello).await?;

        let their_auth = self.recv().await?;
        let challenge = match their_auth {
            Message::Auth { challenge, .. } => challenge,
            _ => return Err(ProtocolError::AuthFailed),
        };

        let response = compute_auth_response(psk, &challenge);
        let auth = Message::Auth {
            challenge: [0u8; 32],
            response,
        };
        self.send(&auth).await?;

        self.peer_id = Some(their_id);
        self.peer_name = Some(their_name.clone());
        Ok((their_id, their_name))
    }

    pub async fn send(&mut self, msg: &Message) -> Result<(), ProtocolError> {
        let data = msg.encode();
        self.stream.write_all(&data).await?;
        Ok(())
    }

    pub async fn recv(&mut self) -> Result<Message, ProtocolError> {
        let mut len_buf = [0u8; 4];
        self.stream.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;

        let mut buf = vec![0u8; 4 + len];
        buf[..4].copy_from_slice(&len_buf);
        self.stream.read_exact(&mut buf[4..]).await?;

        Message::decode(&buf)
    }

    pub fn peer_id(&self) -> Option<Uuid> {
        self.peer_id
    }

    pub async fn run(
        mut self,
        events_tx: mpsc::Sender<PeerEvent>,
        mut clipboard_rx: mpsc::Receiver<(String, u64)>,
    ) {
        let peer_id = match self.peer_id {
            Some(id) => id,
            None => return,
        };
        let peer_name = self.peer_name.clone().unwrap_or_default();

        let _ = events_tx
            .send(PeerEvent::Connected {
                id: peer_id,
                name: peer_name,
            })
            .await;

        loop {
            tokio::select! {
                result = self.recv() => {
                    match result {
                        Ok(Message::Clipboard { content, timestamp }) => {
                            debug!(peer = %peer_id, "received clipboard");
                            let _ = events_tx
                                .send(PeerEvent::Clipboard { content, timestamp })
                                .await;
                        }
                        Ok(Message::Ping) => {
                            let _ = self.send(&Message::Pong).await;
                        }
                        Ok(Message::Pong) => {}
                        Ok(_) => {}
                        Err(e) => {
                            info!(peer = %peer_id, error = %e, "peer disconnected");
                            break;
                        }
                    }
                }
                Some((content, timestamp)) = clipboard_rx.recv() => {
                    let msg = Message::Clipboard { content, timestamp };
                    if self.send(&msg).await.is_err() {
                        break;
                    }
                }
            }
        }

        let _ = events_tx.send(PeerEvent::Disconnected { id: peer_id }).await;
    }
}
