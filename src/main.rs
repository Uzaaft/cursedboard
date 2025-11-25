mod discovery;
mod peer;
mod protocol;
mod trust;

use arboard::Clipboard;
use clap::Parser;
use discovery::Discovery;
use peer::{PeerConnection, PeerEvent};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, Mutex};
use tracing::{error, info, warn};
use trust::{Instance, TrustStore};
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "cursedboard", about = "Zero-config clipboard sync")]
struct Args {
    #[arg(short, long, default_value = "cursedboard")]
    name: String,

    #[arg(short, long, default_value = "42069")]
    port: u16,

    #[arg(long, env = "CURSEDBOARD_PSK", default_value = "cursedboard")]
    psk: String,

    #[arg(short, long, default_value = "500")]
    poll_ms: u64,
}

type ClipboardTx = mpsc::Sender<(String, u64)>;
type PeerMap = Arc<Mutex<HashMap<Uuid, ClipboardTx>>>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("cursedboard=info".parse()?),
        )
        .init();

    let args = Args::parse();
    let instance = Instance::load_or_create()?;
    let trust_store = Arc::new(Mutex::new(TrustStore::load()?));

    info!(id = %instance.id, name = %args.name, port = %args.port, "starting cursedboard");

    let (peer_events_tx, mut peer_events_rx) = mpsc::channel::<PeerEvent>(32);
    let (discovered_tx, mut discovered_rx) = mpsc::channel(32);

    let discovery = Discovery::new(instance.id, args.name.clone(), args.port)?;
    discovery.register()?;
    discovery.browse(discovered_tx)?;

    let listener = TcpListener::bind(("0.0.0.0", args.port)).await?;
    info!(port = %args.port, "listening for connections");

    let peers: PeerMap = Arc::new(Mutex::new(HashMap::new()));
    let last_content = Arc::new(Mutex::new(String::new()));
    let clipboard = Arc::new(Mutex::new(Clipboard::new()?));

    let peers_clone = peers.clone();
    let psk = args.psk.clone();
    let name = args.name.clone();
    let id = instance.id;
    let events_tx = peer_events_tx.clone();
    let trust_clone = trust_store.clone();

    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    info!(%addr, "incoming connection");
                    let mut conn = PeerConnection::from_stream(stream);

                    match conn.handshake_inbound(id, &name, &psk).await {
                        Ok((peer_id, peer_name)) => {
                            let mut trust = trust_clone.lock().await;
                            if !trust.is_trusted(&peer_id) {
                                info!(%peer_id, %peer_name, "trusting new peer");
                                trust.trust(peer_id, peer_name.clone());
                                let _ = trust.save();
                            }
                            drop(trust);

                            let (clipboard_tx, clipboard_rx) = mpsc::channel(16);
                            peers_clone.lock().await.insert(peer_id, clipboard_tx);

                            let tx = events_tx.clone();
                            tokio::spawn(async move {
                                conn.run(tx, clipboard_rx).await;
                            });
                        }
                        Err(e) => {
                            warn!(%addr, error = %e, "handshake failed");
                        }
                    }
                }
                Err(e) => {
                    error!(error = %e, "accept failed");
                }
            }
        }
    });

    let peers_clone = peers.clone();
    let psk = args.psk.clone();
    let name = args.name.clone();
    let id = instance.id;
    let events_tx = peer_events_tx.clone();
    let trust_clone = trust_store.clone();

    tokio::spawn(async move {
        while let Some(peer) = discovered_rx.recv().await {
            info!(id = %peer.id, name = %peer.name, addr = %peer.addr, "discovered peer");

            if peers_clone.lock().await.contains_key(&peer.id) {
                continue;
            }

            let mut conn = match PeerConnection::connect(peer.addr).await {
                Ok(c) => c,
                Err(e) => {
                    warn!(peer = %peer.id, error = %e, "failed to connect");
                    continue;
                }
            };

            match conn.handshake_outbound(id, &name, &psk).await {
                Ok((peer_id, peer_name)) => {
                    let mut trust = trust_clone.lock().await;
                    if !trust.is_trusted(&peer_id) {
                        info!(%peer_id, %peer_name, "trusting new peer");
                        trust.trust(peer_id, peer_name.clone());
                        let _ = trust.save();
                    }
                    drop(trust);

                    let (clipboard_tx, clipboard_rx) = mpsc::channel(16);
                    peers_clone.lock().await.insert(peer_id, clipboard_tx);

                    let tx = events_tx.clone();
                    tokio::spawn(async move {
                        conn.run(tx, clipboard_rx).await;
                    });
                }
                Err(e) => {
                    warn!(peer = %peer.id, error = %e, "handshake failed");
                }
            }
        }
    });

    let peers_clone = peers.clone();
    let last_clone = last_content.clone();
    let clipboard_clone = clipboard.clone();
    let poll_interval = Duration::from_millis(args.poll_ms);

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(poll_interval);
        loop {
            interval.tick().await;

            let content = {
                let mut cb = clipboard_clone.lock().await;
                cb.get_text().unwrap_or_default()
            };

            let mut last = last_clone.lock().await;
            if content != *last && !content.is_empty() {
                *last = content.clone();
                drop(last);

                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;

                let peers = peers_clone.lock().await;
                for (id, tx) in peers.iter() {
                    if tx.send((content.clone(), timestamp)).await.is_err() {
                        warn!(peer = %id, "failed to send clipboard");
                    }
                }
            }
        }
    });

    let clipboard_clone = clipboard.clone();
    let last_clone = last_content.clone();

    while let Some(event) = peer_events_rx.recv().await {
        match event {
            PeerEvent::Connected { id, name } => {
                info!(%id, %name, "peer connected");
            }
            PeerEvent::Clipboard { content, timestamp } => {
                info!(len = content.len(), %timestamp, "received clipboard");
                let mut last = last_clone.lock().await;
                *last = content.clone();
                drop(last);

                let mut cb = clipboard_clone.lock().await;
                if let Err(e) = cb.set_text(&content) {
                    error!(error = %e, "failed to set clipboard");
                }
            }
            PeerEvent::Disconnected { id } => {
                info!(%id, "peer disconnected");
                peers.lock().await.remove(&id);
            }
        }
    }

    Ok(())
}
