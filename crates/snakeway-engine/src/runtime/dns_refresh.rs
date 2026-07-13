use crate::runtime::types::UpstreamTcpRuntime;
use crate::runtime::{RuntimeState, UpstreamRuntime};
use arc_swap::ArcSwap;
use std::net::ToSocketAddrs;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, warn};

pub(crate) async fn run_dns_refresh(state: Arc<ArcSwap<RuntimeState>>, interval: Duration) {
    loop {
        tokio::time::sleep(interval).await;
        refresh_all(&state);
    }
}

fn refresh_all(state: &ArcSwap<RuntimeState>) {
    let snapshot = state.load();
    for service in snapshot.services.values() {
        for upstream in &service.upstreams {
            if let UpstreamRuntime::Tcp(tcp) = upstream {
                if tcp.host.parse::<std::net::IpAddr>().is_ok() {
                    continue;
                }
                refresh_one(tcp);
            }
        }
    }
}

fn refresh_one(tcp: &UpstreamTcpRuntime) {
    match (tcp.host.as_str(), tcp.port).to_socket_addrs() {
        Ok(mut addrs) => {
            if let Some(new_addr) = addrs.next() {
                let old = tcp.resolved_addr.load();
                if old != new_addr {
                    debug!(
                        host = %tcp.host,
                        old = %old,
                        new = %new_addr,
                        "dns refresh: upstream address changed"
                    );
                }
                tcp.resolved_addr.store(new_addr);
            }
        }
        Err(e) => {
            warn!(
                host = %tcp.host,
                error = %e,
                "dns refresh: resolution failed, keeping previous address"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::types::{ResolvedAddr, UpstreamId};

    fn make_tcp_runtime(
        host: &str,
        port: u16,
        resolved: std::net::SocketAddr,
    ) -> UpstreamTcpRuntime {
        UpstreamTcpRuntime {
            id: UpstreamId(0),
            host: host.to_string(),
            port,
            resolved_addr: ResolvedAddr::new(resolved),
            use_tls: false,
            sni: String::new(),
            weight: 1,
            verify: false,
            ca: None,
            group_key: 0,
        }
    }

    #[test]
    fn refresh_one_with_unresolvable_hostname_preserves_old_address() {
        // Arrange
        let original: std::net::SocketAddr = "1.2.3.4:8080".parse().unwrap();
        let tcp = make_tcp_runtime("this-hostname-should-not-resolve.invalid", 8080, original);

        // Act
        refresh_one(&tcp);

        // Assert
        assert_eq!(tcp.resolved_addr.load(), original);
    }

    #[test]
    fn refresh_one_with_localhost_updates_address() {
        // Arrange
        let placeholder: std::net::SocketAddr = "0.0.0.0:80".parse().unwrap();
        let tcp = make_tcp_runtime("localhost", 80, placeholder);

        // Act
        refresh_one(&tcp);

        // Assert
        let refreshed = tcp.resolved_addr.load();
        assert_ne!(refreshed, placeholder);
    }
}
