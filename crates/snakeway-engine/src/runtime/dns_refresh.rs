use crate::runtime::types::UpstreamTcpRuntime;
use crate::runtime::{RuntimeState, UpstreamRuntime};
use arc_swap::ArcSwap;
use std::net::ToSocketAddrs;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, warn};

/// Bounds how long the refresh waits for one blocking getaddrinfo call.
/// The spawned thread itself stays busy until the resolver returns.
const RESOLVE_TIMEOUT: Duration = Duration::from_secs(10);

pub async fn run_dns_refresh(state: Arc<ArcSwap<RuntimeState>>, interval: Duration) {
    loop {
        tokio::time::sleep(interval).await;
        refresh_all(&state).await;
    }
}

async fn refresh_all(state: &ArcSwap<RuntimeState>) {
    let snapshot = state.load();
    for service in snapshot.services.values() {
        for upstream in &service.upstreams {
            if let UpstreamRuntime::Tcp(tcp) = upstream {
                if tcp.host.parse::<std::net::IpAddr>().is_ok() {
                    continue;
                }
                refresh_one(tcp).await;
            }
        }
    }
}

async fn refresh_one(tcp: &UpstreamTcpRuntime) {
    // getaddrinfo blocks its calling thread for the resolver timeout, which
    // would otherwise hold the single worker control runtime and stall every
    // other control plane task with it.
    let host = tcp.host.clone();
    let port = tcp.port;
    let resolved = tokio::time::timeout(
        RESOLVE_TIMEOUT,
        tokio::task::spawn_blocking(move || (host, port).to_socket_addrs().map(|mut a| a.next())),
    )
    .await;

    let new_addr = match resolved {
        Ok(Ok(Ok(Some(new_addr)))) => new_addr,
        Ok(Ok(Ok(None))) => {
            warn!(host = %tcp.host, "dns refresh: resolver returned no addresses, keeping previous address");
            return;
        }
        Ok(Ok(Err(e))) => {
            warn!(host = %tcp.host, error = %e, "dns refresh: resolution failed, keeping previous address");
            return;
        }
        Ok(Err(e)) => {
            warn!(host = %tcp.host, error = %e, "dns refresh: resolve task failed, keeping previous address");
            return;
        }
        Err(_) => {
            warn!(host = %tcp.host, timeout = ?RESOLVE_TIMEOUT, "dns refresh: resolution timed out, keeping previous address");
            return;
        }
    };

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
            id: UpstreamId::new(0),
            host: host.to_string(),
            port,
            resolved_addr: ResolvedAddr::new(resolved),
            tls: None,
            weight: 1,
        }
    }

    #[test]
    fn refresh_one_with_unresolvable_hostname_preserves_old_address() {
        // Arrange
        let original: std::net::SocketAddr = "1.2.3.4:8080".parse().unwrap();
        let tcp = make_tcp_runtime("this-hostname-should-not-resolve.invalid", 8080, original);
        let rt = tokio::runtime::Runtime::new().unwrap();

        // Act
        rt.block_on(refresh_one(&tcp));

        // Assert
        assert_eq!(tcp.resolved_addr.load(), original);
    }

    #[test]
    fn refresh_one_with_localhost_updates_address() {
        // Arrange
        let placeholder: std::net::SocketAddr = "0.0.0.0:80".parse().unwrap();
        let tcp = make_tcp_runtime("localhost", 80, placeholder);
        let rt = tokio::runtime::Runtime::new().unwrap();

        // Act
        rt.block_on(refresh_one(&tcp));

        // Assert
        let refreshed = tcp.resolved_addr.load();
        assert_ne!(refreshed, placeholder);
    }
}
