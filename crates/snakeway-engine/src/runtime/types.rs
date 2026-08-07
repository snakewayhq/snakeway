use crate::execution::device::core::DeviceRegistry;
use crate::execution::route::Router;
use arc_swap::ArcSwap;
use pingora::protocols::tls::CaType;
use snakeway_acme::SniRegistry;
use snakeway_conf::types::{CircuitBreakerConfig, HealthCheckConfig, LoadBalancingStrategy};
use std::collections::HashMap;
use std::hash::Hash;
use std::net::SocketAddr;
use std::sync::Arc;

pub struct RuntimeState {
    pub tls: Option<TlsRuntime>,
    pub routers: HashMap<Arc<str>, Router>,
    pub devices: DeviceRegistry,
    pub services: HashMap<String, ServiceRuntime>,
}

/// TlsRuntime encapsulates the state of TLS configuration.
/// It is reloadable independent of RuntimeState (hence the ArcSwap).
/// This is because it is reloadable not just during a config reload,
/// but also when the cert store is updated.
pub struct TlsRuntime {
    /// Represent an SNI and a parsed certificate.
    pub sni_map: Arc<SniRegistry>,
}

/// The upstreams and load balancing strategy for one service, along with the selection
/// behavior over them.
/// `RuntimeState` holds these and replaces them wholesale on reload.
pub struct ServiceRuntime {
    pub strategy: LoadBalancingStrategy,
    pub upstreams: Vec<UpstreamRuntime>,
    pub circuit_breaker_cfg: CircuitBreakerConfig,
    pub health_check_cfg: HealthCheckConfig,
    #[allow(dead_code)] // useful for debugger inspection
    pub listener: Option<Arc<str>>,
}

#[derive(Debug, Clone)]
pub enum UpstreamRuntime {
    Tcp(UpstreamTcpRuntime),
    Unix(UpstreamUnixRuntime),
}

impl UpstreamRuntime {
    pub fn id(&self) -> UpstreamId {
        match self {
            UpstreamRuntime::Tcp(u) => u.id,
            UpstreamRuntime::Unix(u) => u.id,
        }
    }

    pub(crate) fn weight(&self) -> u32 {
        match self {
            UpstreamRuntime::Tcp(u) => u.weight,
            UpstreamRuntime::Unix(u) => u.weight,
        }
    }

    pub fn use_tls(&self) -> bool {
        match self {
            UpstreamRuntime::Tcp(u) => u.use_tls,
            UpstreamRuntime::Unix(u) => u.use_tls,
        }
    }

    pub fn authority(&self) -> String {
        match self {
            UpstreamRuntime::Tcp(u) => {
                format!("{}:{}", u.host, u.port)
            }
            UpstreamRuntime::Unix(u) => {
                // Logical authority - must exist, even over UDS
                u.sni.clone()
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct UpstreamId(pub(crate) u32);

impl UpstreamId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    pub fn as_u32(&self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, Hash)]
pub(crate) enum UpstreamAddr {
    Tcp { host: String, port: u16 },
    Unix { path: String },
}

/// A DNS-resolved socket address that can be atomically refreshed
/// by the background DNS re-resolver without rebuilding RuntimeState.
pub struct ResolvedAddr(ArcSwap<SocketAddr>);

impl ResolvedAddr {
    pub fn new(addr: SocketAddr) -> Self {
        Self(ArcSwap::from_pointee(addr))
    }

    pub(crate) fn load(&self) -> SocketAddr {
        **self.0.load()
    }

    pub(crate) fn store(&self, addr: SocketAddr) {
        self.0.store(Arc::new(addr));
    }
}

impl Clone for ResolvedAddr {
    fn clone(&self) -> Self {
        Self(ArcSwap::from_pointee(self.load()))
    }
}

impl std::fmt::Debug for ResolvedAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ResolvedAddr({})", self.load())
    }
}

#[derive(Debug, Clone)]
pub struct UpstreamTcpRuntime {
    pub id: UpstreamId,
    pub host: String,
    pub port: u16,
    pub resolved_addr: ResolvedAddr,
    pub use_tls: bool,
    pub sni: String,
    pub weight: u32,
    pub verify: bool,
    /// Preloaded when the runtime snapshot is created.
    pub ca: Option<Arc<CaType>>,
    /// Precomputed when the runtime snapshot is created.
    pub group_key: u64,
}

impl UpstreamTcpRuntime {
    pub fn http_peer_addr(&self) -> SocketAddr {
        self.resolved_addr.load()
    }
}

#[derive(Debug, Clone)]
pub struct UpstreamUnixRuntime {
    pub id: UpstreamId,
    pub path: String,
    pub use_tls: bool,
    pub sni: String,
    pub weight: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolved_addr_load_returns_initial_value() {
        // Arrange
        let addr: SocketAddr = "10.0.0.1:8080".parse().unwrap();

        // Act
        let resolved = ResolvedAddr::new(addr);

        // Assert
        assert_eq!(resolved.load(), addr);
    }

    #[test]
    fn resolved_addr_store_updates_value() {
        // Arrange
        let original: SocketAddr = "10.0.0.1:8080".parse().unwrap();
        let updated: SocketAddr = "10.0.0.2:8080".parse().unwrap();
        let resolved = ResolvedAddr::new(original);

        // Act
        resolved.store(updated);

        // Assert
        assert_eq!(resolved.load(), updated);
    }

    #[test]
    fn resolved_addr_clone_is_independent() {
        // Arrange
        let addr: SocketAddr = "10.0.0.1:8080".parse().unwrap();
        let resolved = ResolvedAddr::new(addr);
        let cloned = resolved.clone();

        // Act
        resolved.store("10.0.0.2:8080".parse().unwrap());

        // Assert
        assert_eq!(cloned.load(), addr);
    }

    #[test]
    fn http_peer_addr_returns_resolved_socket_addr() {
        // Arrange
        let addr: SocketAddr = "192.168.1.1:3000".parse().unwrap();
        let tcp = UpstreamTcpRuntime {
            id: UpstreamId(0),
            host: "my-service".to_string(),
            port: 3000,
            resolved_addr: ResolvedAddr::new(addr),
            use_tls: false,
            sni: String::new(),
            weight: 1,
            verify: false,
            ca: None,
            group_key: 0,
        };

        // Act
        let peer_addr = tcp.http_peer_addr();

        // Assert
        assert_eq!(peer_addr, addr);
    }
}
