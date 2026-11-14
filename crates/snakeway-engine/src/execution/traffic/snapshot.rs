use crate::execution::traffic::types::*;
use crate::runtime::{RuntimeState, UpstreamId, UpstreamRuntime};
use opentelemetry::KeyValue;
use snakeway_conf::types::LoadBalancingStrategy;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct UpstreamSnapshot {
    pub endpoint: UpstreamRuntime,
    pub(crate) latency: Option<LatencyStats>,
    pub(crate) weight: u32,
    pub(crate) metric_attributes: [KeyValue; 2],
}

impl UpstreamSnapshot {
    /// The `service` and `upstream` metric attributes, built once per snapshot.
    /// Their values are reference counted, so a clone does not allocate.
    pub fn metric_attributes(&self) -> &[KeyValue] {
        &self.metric_attributes
    }
}

#[derive(Debug, Clone)]
pub struct ServiceSnapshot {
    #[allow(dead_code)] // useful for debugger inspection
    pub(crate) service_id: ServiceId,
    pub(crate) strategy: LoadBalancingStrategy,
    pub upstreams: Vec<UpstreamSnapshot>,
    pub(crate) circuit_breaker_cfg: snakeway_conf::types::CircuitBreakerConfig,
    pub(crate) health_check_cfg: snakeway_conf::types::HealthCheckConfig,
    pub(crate) metric_service: KeyValue,
}

impl ServiceSnapshot {
    /// The `service` metric attribute, built once per snapshot.
    pub fn metric_service(&self) -> &KeyValue {
        &self.metric_service
    }
}

pub(crate) fn service_metric_attribute(service: &Arc<str>) -> KeyValue {
    KeyValue::new("service", Arc::clone(service))
}

pub(crate) fn upstream_metric_attributes(
    service: &Arc<str>,
    upstream: UpstreamId,
) -> [KeyValue; 2] {
    [
        service_metric_attribute(service),
        KeyValue::new("upstream", Arc::<str>::from(upstream.as_u32().to_string())),
    ]
}

/// Immutable, control-plane snapshot of traffic topology and health.
///
/// Safe to read from the request hot path.
/// Updated only by reload, health checks, or discovery.
#[derive(Debug, Clone, Default)]
pub struct TrafficSnapshot {
    pub services: HashMap<ServiceId, ServiceSnapshot>,
}

impl TrafficSnapshot {
    pub fn from_runtime(state: &RuntimeState) -> Self {
        let mut services = HashMap::new();

        for (name, svc) in &state.services {
            let service_name: Arc<str> = Arc::from(name.as_str());

            let upstreams = svc
                .upstreams
                .iter()
                .map(|u| UpstreamSnapshot {
                    endpoint: u.clone(),
                    latency: None,
                    weight: u.weight(),
                    metric_attributes: upstream_metric_attributes(&service_name, u.id()),
                })
                .collect::<Vec<_>>();

            services.insert(
                ServiceId(Arc::clone(&service_name)),
                ServiceSnapshot {
                    service_id: ServiceId(Arc::clone(&service_name)),
                    strategy: svc.strategy,
                    upstreams,
                    circuit_breaker_cfg: svc.circuit_breaker_cfg.clone(),
                    health_check_cfg: svc.health_check_cfg.clone(),
                    metric_service: service_metric_attribute(&service_name),
                },
            );
        }

        TrafficSnapshot { services }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upstream_metric_attributes_name_the_service_and_the_numeric_upstream_id() {
        // Arrange
        let service: Arc<str> = Arc::from("api");

        // Act
        let attributes = upstream_metric_attributes(&service, UpstreamId(4_000_000_001));

        // Assert
        assert_eq!(attributes[0].key.as_str(), "service");
        assert_eq!(attributes[0].value.as_str(), "api");
        assert_eq!(attributes[1].key.as_str(), "upstream");
        assert_eq!(attributes[1].value.as_str(), "4000000001");
    }

    #[test]
    fn cloning_cached_metric_attributes_does_not_allocate() {
        // Arrange
        let service: Arc<str> = Arc::from("api");
        let attributes = upstream_metric_attributes(&service, UpstreamId(7));
        let service_attribute = service_metric_attribute(&service);

        // Act
        let (clones, allocations) = crate::alloc_count::count_allocations(|| {
            (
                attributes[0].clone(),
                attributes[1].clone(),
                service_attribute.clone(),
            )
        });

        // Assert
        assert_eq!(allocations, 0);
        assert_eq!(clones.0.value.as_str(), "api");
        assert_eq!(clones.1.value.as_str(), "7");
        assert_eq!(clones.2.value.as_str(), "api");
    }
}
