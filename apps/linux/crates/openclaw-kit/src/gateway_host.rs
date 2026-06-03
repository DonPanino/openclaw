//! Host classification for connection mode (LAN vs SSH remote).

/// True when the gateway is reachable without SSH tunnel (loopback, RFC1918, link-local, mDNS).
pub fn is_direct_lan_host(host: &str) -> bool {
    let host = host
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']');
    if host.eq_ignore_ascii_case("localhost") || host == "127.0.0.1" || host == "::1" {
        return true;
    }
    if host.ends_with(".local") {
        return true;
    }
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        return match ip {
            std::net::IpAddr::V4(v4) => {
                v4.is_loopback() || v4.is_private() || v4.is_link_local()
            }
            std::net::IpAddr::V6(v6) => {
                v6.is_loopback() || v6.is_unique_local() || (v6.segments()[0] & 0xffc0) == 0xfe80
            }
        };
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_lan_hosts() {
        assert!(is_direct_lan_host("192.168.1.10"));
        assert!(is_direct_lan_host("10.0.0.2"));
        assert!(is_direct_lan_host("127.0.0.1"));
        assert!(is_direct_lan_host("mybox.local"));
    }

    #[test]
    fn rejects_public_hostnames() {
        assert!(!is_direct_lan_host("gateway.example.com"));
    }
}
