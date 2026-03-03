use crate::cert_manager::SniRegistry;
use crate::conf::types::{CircuitBreakerConfig, HealthCheckConfig, LoadBalancingStrategy};
use crate::device::core::registry::DeviceRegistry;
use crate::route::Router;
use pingora::protocols::tls::CaType;
use std::collections::HashMap;
use std::hash::Hash;
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

/// ServiceRuntime encapsulates the state of a service, including its
/// upstream(s) and load balancing strategy.
/// It is not just a collection of data, but also a behavioral unit distinct
/// from RuntimeState.
pub struct ServiceRuntime {
    pub strategy: LoadBalancingStrategy,
    pub upstreams: Vec<UpstreamRuntime>,
    pub circuit_breaker_cfg: CircuitBreakerConfig,
    pub health_check_cfg: HealthCheckConfig,
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

    pub fn weight(&self) -> u32 {
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
pub struct UpstreamId(pub u32);

#[derive(Debug, Clone, Hash)]
pub enum UpstreamAddr {
    Tcp { host: String, port: u16 },
    Unix { path: String },
}

#[derive(Debug, Clone)]
pub struct UpstreamTcpRuntime {
    pub id: UpstreamId,
    pub host: String,
    pub port: u16,
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
    pub fn http_peer_addr(&self) -> (&str, u16) {
        (self.host.as_str(), self.port)
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
