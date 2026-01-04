use crate::conf::types::{CircuitBreakerConfig, HealthCheckConfig, LoadBalancingStrategy};
use crate::device::core::registry::DeviceRegistry;
use crate::route::Router;
use std::collections::HashMap;
use std::hash::Hash;

pub struct RuntimeState {
    pub router: Router,
    pub devices: DeviceRegistry,
    pub services: HashMap<String, ServiceRuntime>,
}

/// ServiceRuntime encapsulates the state of a service, including its upstream(s) and load balancing strategy.
/// It is not just a collection of data, but also a behavioral unit distinct from RuntimeState.
pub struct ServiceRuntime {
    pub strategy: LoadBalancingStrategy,
    pub upstreams: Vec<UpstreamRuntime>,
    pub circuit_breaker_cfg: CircuitBreakerConfig,
    pub health_check_cfg: HealthCheckConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct UpstreamId(pub u32);

#[derive(Debug, Clone)]
pub struct UpstreamRuntime {
    pub id: UpstreamId,
    pub host: String,
    pub port: u16,
    pub use_tls: bool,
    pub sni: String,
    pub weight: u32,
}
