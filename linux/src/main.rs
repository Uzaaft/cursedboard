mod clipboard_provider;

use clipboard_provider::LinuxClipboardProvider;
use log::{debug, error, info};
use shared::{
    cli::Cli,
    connection::ConnectionManager,
    discovery::DiscoveryManager,
    instance::Instance,
    protocol::{encode_message, ClipboardUpdateMessage, Message},
};
use std::time::Duration;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse_args();

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(
        cli.log_level.as_deref().unwrap_or("info"),
    ))
    .init();

    info!("Starting cursedboard (Linux)");

    let mut instance = Instance::load_or_create()?;
    if let Some(group) = cli.group.clone() {
        instance.group = Some(group);
    }

    let port = cli.port;
    let bind_addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&bind_addr).await?;
    info!("Listening on {}", bind_addr);

    let mut connection_manager = ConnectionManager::new(instance.clone(), cli.psk.clone());

    if let Some(pair_seconds) = cli.pair {
        connection_manager.enable_pairing(Duration::from_secs(pair_seconds));
    }

    connection_manager.accept_connections(listener).await?;

    if !cli.no_discovery {
        let mut discovery = DiscoveryManager::new(
            instance.id,
            instance.device_name.clone(),
            port,
            instance.get_group(),
        )?;

        info!("mDNS discovery enabled");

        let conn_mgr_clone = connection_manager.clone();
        tokio::spawn(async move {
            while let Some(peer) = discovery.next_peer().await {
                conn_mgr_clone.handle_discovered_peer(peer).await;
            }
        });
    } else {
        info!("Discovery disabled via --no-discovery");
    }

    use shared::clipboard::ClipboardProvider;

    let mut provider = LinuxClipboardProvider::new()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    let (outbound_tx, mut outbound_rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();

    // Spawn clipboard monitor task
    tokio::task::spawn_blocking(move || {
        let mut provider_monitor = LinuxClipboardProvider::new().unwrap();
        let mut last_content: Option<String> = None;
        loop {
            std::thread::sleep(Duration::from_millis(500));

            match provider_monitor.get_text() {
                Ok(content) => {
                    if last_content.as_ref() != Some(&content) && !content.is_empty() {
                        debug!("Local clipboard changed: {} bytes", content.len());
                        last_content = Some(content.clone());

                        let msg = Message::ClipboardUpdate(ClipboardUpdateMessage::new(content));
                        if let Ok(encoded) = encode_message(&msg) {
                            let _ = outbound_tx.send(encoded);
                        }
                    }
                }
                Err(e) => {
                    error!("Error reading clipboard: {}", e);
                }
            }
        }
    });

    // Handle both incoming clipboard updates and outgoing ones
    loop {
        tokio::select! {
            Some(content) = connection_manager.next_clipboard_update() => {
                debug!("Received clipboard update: {} bytes", content.len());
                if let Err(e) = provider.set_text(content) {
                    error!("Failed to set clipboard: {}", e);
                }
            }
            Some(_encoded) = outbound_rx.recv() => {
                // TODO: Broadcast to connected peers
                // For now, we'd need to add a method to ConnectionManager to send data
                debug!("Local clipboard change, ready to broadcast");
            }
        }
    }
}
