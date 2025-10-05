use log::{debug, error, info, warn};
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

const SERVICE_TYPE: &str = "_cursedboard._tcp.local.";

#[derive(Debug, Clone)]
pub struct DiscoveredPeer {
    pub instance_id: Uuid,
    pub device_name: String,
    pub address: SocketAddr,
    pub group: String,
    pub version: String,
    pub features: Vec<String>,
}

pub struct DiscoveryManager {
    mdns: Arc<ServiceDaemon>,
    receiver: mpsc::UnboundedReceiver<DiscoveredPeer>,
}

impl DiscoveryManager {
    pub fn new(
        instance_id: Uuid,
        device_name: String,
        port: u16,
        group: String,
    ) -> Result<Self, DiscoveryError> {
        let mdns = ServiceDaemon::new().map_err(|e| {
            DiscoveryError::MdnsInit(format!("Failed to initialize mDNS: {}", e))
        })?;

        let (tx, rx) = mpsc::unbounded_channel();

        let mdns_arc = Arc::new(mdns);

        let service_name = format!("cursedboard-{}", instance_id);
        let hostname = format!("{}.local.", device_name.replace(' ', "-").to_lowercase());
        
        let mut properties = HashMap::new();
        properties.insert("id".to_string(), instance_id.to_string());
        properties.insert("name".to_string(), device_name.clone());
        properties.insert("ver".to_string(), env!("CARGO_PKG_VERSION").to_string());
        properties.insert("features".to_string(), "text".to_string());
        properties.insert("group".to_string(), group.clone());

        let service_info = ServiceInfo::new(
            SERVICE_TYPE,
            &service_name,
            &hostname,
            "",
            port,
            Some(properties),
        )
        .map_err(|e| DiscoveryError::ServiceCreate(format!("Failed to create service: {}", e)))?;

        mdns_arc
            .register(service_info)
            .map_err(|e| DiscoveryError::Register(format!("Failed to register service: {}", e)))?;

        info!("Registered mDNS service: {} on port {}", service_name, port);

        let mdns_clone = mdns_arc.clone();
        let self_id = instance_id;
        tokio::spawn(async move {
            let receiver = mdns_clone.browse(SERVICE_TYPE).unwrap();
            
            while let Ok(event) = receiver.recv_async().await {
                match event {
                    ServiceEvent::ServiceResolved(info) => {
                        debug!("Service resolved: {:?}", info.get_fullname());
                        
                        if let Some(peer) = Self::parse_service_info(&info, self_id) {
                            if peer.instance_id != self_id {
                                info!("Discovered peer: {} ({})", peer.device_name, peer.instance_id);
                                let _ = tx.send(peer);
                            } else {
                                debug!("Ignoring self-discovery");
                            }
                        }
                    }
                    ServiceEvent::SearchStarted(_) => {
                        debug!("mDNS search started");
                    }
                    ServiceEvent::ServiceFound(_, _) => {
                        debug!("Service found (awaiting resolution)");
                    }
                    ServiceEvent::ServiceRemoved(_, fullname) => {
                        debug!("Service removed: {}", fullname);
                    }
                    _ => {}
                }
            }
        });

        Ok(DiscoveryManager {
            mdns: mdns_arc,
            receiver: rx,
        })
    }

    fn parse_service_info(info: &ServiceInfo, self_id: Uuid) -> Option<DiscoveredPeer> {
        let properties = info.get_properties();
        
        let instance_id = properties
            .get("id")
            .and_then(|v| Uuid::parse_str(v.val_str()).ok())?;

        if instance_id == self_id {
            return None;
        }

        let device_name = properties
            .get("name")
            .map(|v| v.val_str())
            .unwrap_or("unknown")
            .to_string();

        let version = properties
            .get("ver")
            .map(|v| v.val_str())
            .unwrap_or("0.1.0")
            .to_string();

        let features = properties
            .get("features")
            .map(|v| v.val_str())
            .unwrap_or("text")
            .split(',')
            .map(String::from)
            .collect();

        let group = properties
            .get("group")
            .map(|v| v.val_str())
            .unwrap_or("default")
            .to_string();

        let addresses = info.get_addresses();
        let port = info.get_port();

        let address = addresses
            .iter()
            .find(|addr| matches!(addr, IpAddr::V4(_)))
            .or_else(|| addresses.iter().next())
            .map(|addr| SocketAddr::new(*addr, port))?;

        Some(DiscoveredPeer {
            instance_id,
            device_name,
            address,
            group,
            version,
            features,
        })
    }

    pub async fn next_peer(&mut self) -> Option<DiscoveredPeer> {
        self.receiver.recv().await
    }

    pub fn shutdown(&self) {
        if let Err(e) = self.mdns.shutdown() {
            warn!("Error shutting down mDNS: {}", e);
        }
    }
}

#[derive(Debug)]
pub enum DiscoveryError {
    MdnsInit(String),
    ServiceCreate(String),
    Register(String),
}

impl std::fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiscoveryError::MdnsInit(msg) => write!(f, "mDNS initialization error: {}", msg),
            DiscoveryError::ServiceCreate(msg) => write!(f, "Service creation error: {}", msg),
            DiscoveryError::Register(msg) => write!(f, "Service registration error: {}", msg),
        }
    }
}

impl std::error::Error for DiscoveryError {}
