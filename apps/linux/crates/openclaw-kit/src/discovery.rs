use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredGateway {
    pub name: String,
    pub host: String,
    pub port: u16,
}

/// Browse for `_openclaw-gw._tcp` via mDNS (best-effort).
pub async fn discover_gateways(timeout: Duration) -> Vec<DiscoveredGateway> {
    let mut found = Vec::new();
    let Ok(service) = mdns_sd::ServiceDaemon::new() else {
        return found;
    };
    let Ok(receiver) = service.browse("_openclaw-gw._tcp.local.") else {
        return found;
    };
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match tokio::time::timeout(remaining, async {
            receiver.recv_async().await
        })
        .await
        {
            Ok(Ok(event)) => {
                if let mdns_sd::ServiceEvent::ServiceResolved(info) = event {
                    let host = info
                        .get_addresses()
                        .iter()
                        .next()
                        .map(|a| a.to_string())
                        .unwrap_or_else(|| info.get_hostname().to_string());
                    let port = info.get_port();
                    found.push(DiscoveredGateway {
                        name: info.get_fullname().to_string(),
                        host,
                        port,
                    });
                }
            }
            _ => break,
        }
    }
    let _ = service.shutdown();
    found
}
