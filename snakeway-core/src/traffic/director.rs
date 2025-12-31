use crate::conf::types::LoadBalancingStrategy;
use crate::traffic::circuit::CircuitState;
use crate::traffic::{
    TrafficManager, algorithms::*, decision::*, snapshot::*, strategy::TrafficStrategy,
};
use once_cell::sync::Lazy;

static FAILOVER: Lazy<Failover> = Lazy::new(Failover::default);
static HASH: Lazy<StickyHash> = Lazy::new(StickyHash::default);
static LEAST_CONNECTIONS: Lazy<LeastConnections> = Lazy::new(LeastConnections::default);
static RANDOM: Lazy<Random> = Lazy::new(Random::default);
static ROUND_ROBIN: Lazy<RoundRobin> = Lazy::new(RoundRobin::default);

#[derive(Debug, Default)]
pub struct TrafficDirector;

impl TrafficDirector {
    pub fn decide(
        &self,
        req: &crate::ctx::RequestCtx,
        snapshot: &TrafficSnapshot,
        service_id: &crate::traffic::types::ServiceId,
        traffic_manager: &TrafficManager,
    ) -> Result<TrafficDecision, TrafficError> {
        let service = snapshot
            .services
            .get(service_id)
            .ok_or(TrafficError::UnknownService)?;

        // Filter eligible upstreams (healthy AND circuit allows)
        let eligible: Vec<_> = service
            .upstreams
            .iter()
            .filter(|u| {
                // Health gate (purely time-based).
                if !traffic_manager
                    .health_status(service_id, &u.endpoint.id)
                    .healthy
                {
                    return false;
                }

                // Circuit gate (state-only, no mutation).
                match traffic_manager.circuit_state(service_id, &u.endpoint.id) {
                    CircuitState::Open => false,
                    CircuitState::Closed | CircuitState::HalfOpen => true,
                }
            })
            .cloned()
            .collect();

        if eligible.is_empty() {
            return Err(TrafficError::NoHealthyUpstreams);
        }

        // Delegate to strategy
        let strategy: &dyn TrafficStrategy = match service.strategy {
            LoadBalancingStrategy::Failover => &*FAILOVER,
            LoadBalancingStrategy::RoundRobin => &*ROUND_ROBIN,
            LoadBalancingStrategy::LeastConnections => &*LEAST_CONNECTIONS,
            LoadBalancingStrategy::StickyHash => &*HASH,
            LoadBalancingStrategy::Random => &*RANDOM,
        };

        if let Some(decision) = strategy.decide(req, service_id, &eligible, traffic_manager) {
            return Ok(decision);
        }

        // Hard fallback: first eligible
        Ok(TrafficDecision {
            upstream_id: eligible[0].endpoint.id,
            reason: DecisionReason::NoStrategyDecision,
            protocol: None,
        })
    }
}

#[derive(Debug)]
pub enum TrafficError {
    UnknownService,
    NoHealthyUpstreams,
}
