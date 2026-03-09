use crate::conf::types::{CircuitBreakerConfig, HealthCheckConfig, LoadBalancingStrategy};
use crate::control_plane::acme::SniRegistry;
use crate::execution::device::core::registry::DeviceRegistry;
use crate::execution::route::Router;
use pingora::protocols::tls::CaType;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;

pub struct RuntimeState {
    pub tls: Option<TlsRuntime>,
    pub(crate) routers: HashMap<Arc<str>, Router>,
    pub(crate) devices: DeviceRegistry,
    pub(crate) services: HashMap<String, ServiceRuntime>,
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
pub(crate) struct ServiceRuntime {
    pub(crate) strategy: LoadBalancingStrategy,
    pub(crate) upstreams: Vec<UpstreamRuntime>,
    pub(crate) circuit_breaker_cfg: CircuitBreakerConfig,
    pub(crate) health_check_cfg: HealthCheckConfig,
    pub(crate) listener: Option<Arc<str>>,
}

#[derive(Debug, Clone)]
pub(crate) enum UpstreamRuntime {
    Tcp(UpstreamTcpRuntime),
    Unix(UpstreamUnixRuntime),
}

impl UpstreamRuntime {
    pub(crate) fn id(&self) -> UpstreamId {
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

    pub(crate) fn use_tls(&self) -> bool {
        match self {
            UpstreamRuntime::Tcp(u) => u.use_tls,
            UpstreamRuntime::Unix(u) => u.use_tls,
        }
    }

    pub(crate) fn authority(&self) -> String {
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
pub(crate) struct UpstreamId(pub(crate) u32);

#[derive(Debug, Clone, Hash)]
pub(crate) enum UpstreamAddr {
    Tcp { host: String, port: u16 },
    Unix { path: String },
}

#[derive(Debug, Clone)]
pub(crate) struct UpstreamTcpRuntime {
    pub(crate) id: UpstreamId,
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) use_tls: bool,
    pub(crate) sni: String,
    pub(crate) weight: u32,
    pub(crate) verify: bool,
    /// Preloaded when the runtime snapshot is created.
    pub(crate) ca: Option<Arc<CaType>>,
    /// Precomputed when the runtime snapshot is created.
    pub(crate) group_key: u64,
}

impl UpstreamTcpRuntime {
    pub(crate) fn http_peer_addr(&self) -> (&str, u16) {
        (self.host.as_str(), self.port)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct UpstreamUnixRuntime {
    pub(crate) id: UpstreamId,
    pub(crate) path: String,
    pub(crate) use_tls: bool,
    pub(crate) sni: String,
    pub(crate) weight: u32,
}
