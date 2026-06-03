use serde::{Deserialize, Serialize};
use std::collections::HashSet;
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
                        name: gateway_display_name(info.get_fullname()),
                        host,
                        port,
                    });
                }
            }
            _ => break,
        }
    }
    let _ = service.shutdown();
    dedupe_gateways(found)
}

/// Short label from an mDNS full service name (strips `._openclaw-gw._tcp.local.` suffix).
pub fn gateway_display_name(fullname: &str) -> String {
    fullname
        .split('.')
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or(fullname)
        .to_string()
}

/// Keep first occurrence per host:port (mDNS can emit duplicates).
pub fn dedupe_gateways(found: Vec<DiscoveredGateway>) -> Vec<DiscoveredGateway> {
    let mut seen = HashSet::new();
    found
        .into_iter()
        .filter(|gw| seen.insert((gw.host.clone(), gw.port)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_name_strips_service_domain() {
        assert_eq!(
            gateway_display_name("mybox._openclaw-gw._tcp.local."),
            "mybox"
        );
    }

    #[test]
    fn dedupe_keeps_first_per_host_port() {
        let list = vec![
            DiscoveredGateway {
                name: "a".into(),
                host: "192.168.1.1".into(),
                port: 18789,
            },
            DiscoveredGateway {
                name: "b".into(),
                host: "192.168.1.1".into(),
                port: 18789,
            },
            DiscoveredGateway {
                name: "c".into(),
                host: "10.0.0.2".into(),
                port: 18789,
            },
        ];
        let out = dedupe_gateways(list);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].name, "a");
    }
}
