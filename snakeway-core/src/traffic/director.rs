use crate::conf::types::LoadBalancingStrategy;
use crate::traffic::{
    TrafficManager, algorithms::*, decision::*, snapshot::*, strategy::TrafficStrategy,
};
use once_cell::sync::Lazy;

static FAILOVER: Lazy<Failover> = Lazy::new(Failover::default);
static HASH: Lazy<StickyHash> = Lazy::new(StickyHash::default);
static LEAST_CONNECTIONS: Lazy<RequestPressure> = Lazy::new(RequestPressure::default);
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

        // Purely filter on health status.
        let mut healthy_candidates: Vec<_> = service
            .upstreams
            .iter()
            .filter(|u| {
                traffic_manager
                    .health_status(service_id, &u.endpoint.id)
                    .healthy
            })
            .cloned()
            .collect();

        if healthy_candidates.is_empty() {
            return Err(TrafficError::NoHealthyUpstreams);
        }

        // Select strategy.
        let strategy: &dyn TrafficStrategy = match service.strategy {
            LoadBalancingStrategy::Failover => &*FAILOVER,
            LoadBalancingStrategy::RoundRobin => &*ROUND_ROBIN,
            LoadBalancingStrategy::RequestPressure => &*LEAST_CONNECTIONS,
            LoadBalancingStrategy::StickyHash => &*HASH,
            LoadBalancingStrategy::Random => &*RANDOM,
        };

        // Pick upstream and circuit admission
        while !healthy_candidates.is_empty() {
            let decision = strategy
                .decide(req, service_id, &healthy_candidates, traffic_manager)
                .unwrap_or_else(|| TrafficDecision {
                    upstream_id: healthy_candidates[0].endpoint.id,
                    reason: DecisionReason::NoStrategyDecision,
                    protocol: None,
                    cb_started: true,
                });

            if traffic_manager.circuit_allows(service_id, &decision.upstream_id) {
                return Ok(decision);
            }

            // Circuit denied: remove and retry.
            healthy_candidates.retain(|u| u.endpoint.id != decision.upstream_id);
        }

        Err(TrafficError::NoHealthyUpstreams)
    }
}

#[derive(Debug)]
pub enum TrafficError {
    UnknownService,
    NoHealthyUpstreams,
}
