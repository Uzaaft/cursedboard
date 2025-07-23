use std::{
    net::TcpStream,
    sync::{mpsc, Arc, Mutex},
    thread,
    time::Duration,
};

use log::{debug, error, info};

use crate::{network::send_clipboard_message, ClipboardMessage};

/// Platform-specific clipboard operations
pub trait ClipboardProvider: Send + 'static {
    /// Get the current clipboard text
    fn get_text(&mut self) -> Result<String, Box<dyn std::error::Error>>;

    /// Set the clipboard text
    fn set_text(&mut self, text: String) -> Result<(), Box<dyn std::error::Error>>;

    /// Check if clipboard has changed (returns new content if changed)
    fn check_changed(&mut self) -> Option<String>;
}

/// Manages clipboard monitoring and synchronization
pub struct ClipboardManager {
    provider: Box<dyn ClipboardProvider>,
    connections: Arc<Mutex<Vec<Arc<Mutex<TcpStream>>>>>,
    update_rx: mpsc::Receiver<String>,
    update_tx: mpsc::Sender<String>,
    monitor_tx: mpsc::Sender<ClipboardEvent>,
}

#[derive(Debug)]
pub enum ClipboardEvent {
    LocalChange(String),
    RemoteChange(String),
    Shutdown,
}

impl ClipboardManager {
    /// Create a new clipboard manager with a platform-specific provider
    pub fn new(
        provider: Box<dyn ClipboardProvider>,
        connections: Arc<Mutex<Vec<Arc<Mutex<TcpStream>>>>>,
    ) -> (Self, mpsc::Receiver<ClipboardEvent>) {
        let (update_tx, update_rx) = mpsc::channel();
        let (monitor_tx, monitor_rx) = mpsc::channel();

        let manager = Self {
            provider,
            connections,
            update_rx,
            update_tx,
            monitor_tx,
        };

        (manager, monitor_rx)
    }

    /// Get a sender for remote clipboard updates
    pub fn get_update_sender(&self) -> mpsc::Sender<String> {
        self.update_tx.clone()
    }

    /// Run the clipboard manager (blocks until shutdown)
    pub fn run(mut self) {
        loop {
            // Check for remote updates with timeout
            match self.update_rx.recv_timeout(Duration::from_millis(50)) {
                Ok(content) => {
                    if let Err(e) = self.provider.set_text(content.clone()) {
                        error!("Failed to set clipboard: {e}");
                    } else {
                        // Notify monitor thread of remote change
                        let _ = self.monitor_tx.send(ClipboardEvent::RemoteChange(content));
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // Check for local clipboard changes
                    if let Some(content) = self.provider.check_changed() {
                        // Broadcast to all connected peers
                        self.broadcast_clipboard_change(&content);

                        // Notify monitor thread of local change
                        let _ = self.monitor_tx.send(ClipboardEvent::LocalChange(content));
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    let _ = self.monitor_tx.send(ClipboardEvent::Shutdown);
                    break;
                }
            }
        }
    }

    fn broadcast_clipboard_change(&self, content: &str) {
        let msg = ClipboardMessage::new(content.to_string());
        let conns = self.connections.lock().unwrap();

        for stream in conns.iter() {
            if let Ok(mut stream) = stream.lock() {
                if let Err(e) = send_clipboard_message(&mut stream, &msg) {
                    error!("Failed to send clipboard: {e}");
                } else {
                    debug!("Sent clipboard content: {} bytes", msg.content.len());
                }
            }
        }
    }
}

/// Connection handler that works with any clipboard provider
pub struct ConnectionHandler {
    clipboard_tx: mpsc::Sender<String>,
    connections: Arc<Mutex<Vec<Arc<Mutex<TcpStream>>>>>,
}

impl ConnectionHandler {
    pub fn new(
        clipboard_tx: mpsc::Sender<String>,
        connections: Arc<Mutex<Vec<Arc<Mutex<TcpStream>>>>>,
    ) -> Self {
        Self {
            clipboard_tx,
            connections,
        }
    }

    pub fn handle_connection(&self, stream: TcpStream, addr: std::net::SocketAddr) {
        use crate::network::handle_incoming_messages;

        // Clone the stream for sending
        let send_stream = match stream.try_clone() {
            Ok(s) => Arc::new(Mutex::new(s)),
            Err(e) => {
                error!("Failed to clone stream: {e}");
                return;
            }
        };

        // Add to connections list
        self.connections.lock().unwrap().push(send_stream.clone());

        // Handle incoming messages
        let clipboard_tx = self.clipboard_tx.clone();
        let result = handle_incoming_messages(stream, move |msg| {
            debug!(
                "Received clipboard content from {}: {} bytes",
                addr,
                msg.content.len()
            );
            if let Err(e) = clipboard_tx.send(msg.content) {
                error!("Failed to send clipboard update: {e}");
                return Err(std::io::Error::other(e));
            }
            Ok(())
        });

        if let Err(e) = result {
            error!("Connection error from {addr}: {e}");
        }

        // Remove from connections list
        self.connections
            .lock()
            .unwrap()
            .retain(|s| !Arc::ptr_eq(s, &send_stream));
    }
}

/// Spawn the clipboard manager in a separate thread
pub fn spawn_clipboard_manager(
    provider: Box<dyn ClipboardProvider>,
    connections: Arc<Mutex<Vec<Arc<Mutex<TcpStream>>>>>,
) -> (mpsc::Sender<String>, mpsc::Receiver<ClipboardEvent>) {
    let (manager, event_rx) = ClipboardManager::new(provider, connections);
    let update_tx = manager.get_update_sender();

    thread::spawn(move || {
        manager.run();
    });

    (update_tx, event_rx)
}
