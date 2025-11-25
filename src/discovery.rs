use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use std::collections::HashSet;
use std::net::SocketAddr;

use thiserror::Error;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};
use uuid::Uuid;

const SERVICE_TYPE: &str = "_cursedboard._tcp.local.";

#[derive(Debug, Error)]
pub enum DiscoveryError {
    #[error("mdns error: {0}")]
    Mdns(#[from] mdns_sd::Error),
}

#[derive(Debug, Clone)]
pub struct Peer {
    pub id: Uuid,
    pub name: String,
    pub addr: SocketAddr,
}

pub struct Discovery {
    daemon: ServiceDaemon,
    instance_id: Uuid,
    name: String,
    port: u16,
}

impl Discovery {
    pub fn new(instance_id: Uuid, name: String, port: u16) -> Result<Self, DiscoveryError> {
        let daemon = ServiceDaemon::new()?;
        Ok(Self {
            daemon,
            instance_id,
            name,
            port,
        })
    }

    pub fn register(&self) -> Result<(), DiscoveryError> {
        let host = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        let service_name = format!("{}_{}", self.name, self.instance_id);
        let service = ServiceInfo::new(
            SERVICE_TYPE,
            &service_name,
            &format!("{}.local.", host),
            (),
            self.port,
            [("id", self.instance_id.to_string().as_str())].as_slice(),
        )?;

        self.daemon.register(service)?;
        info!(name = %self.name, port = %self.port, "registered mDNS service");
        Ok(())
    }

    pub fn browse(&self, tx: mpsc::Sender<Peer>) -> Result<(), DiscoveryError> {
        let receiver = self.daemon.browse(SERVICE_TYPE)?;
        let own_id = self.instance_id;

        tokio::spawn(async move {
            let mut seen: HashSet<Uuid> = HashSet::new();
            
            loop {
                match receiver.recv() {
                    Ok(event) => match event {
                        ServiceEvent::ServiceResolved(info) => {
                            let id = match info.get_properties().get("id") {
                                Some(prop) => match prop.val_str().parse::<Uuid>() {
                                    Ok(id) => id,
                                    Err(_) => continue,
                                },
                                None => continue,
                            };

                            if id == own_id || seen.contains(&id) {
                                continue;
                            }

                            let addr = match info.get_addresses().iter().next() {
                                Some(ip) => SocketAddr::new(ip.to_ip_addr(), info.get_port()),
                                None => continue,
                            };

                            let name = info
                                .get_fullname()
                                .split('_')
                                .next()
                                .unwrap_or("unknown")
                                .to_string();

                            seen.insert(id);
                            let peer = Peer { id, name, addr };
                            debug!(?peer, "discovered peer");

                            if tx.send(peer).await.is_err() {
                                break;
                            }
                        }
                        ServiceEvent::ServiceRemoved(_, fullname) => {
                            debug!(fullname, "peer removed");
                        }
                        _ => {}
                    },
                    Err(e) => {
                        warn!("mdns browse error: {}", e);
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    pub fn shutdown(self) -> Result<(), DiscoveryError> {
        self.daemon.shutdown()?;
        Ok(())
    }
}
