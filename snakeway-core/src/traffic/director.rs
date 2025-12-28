use crate::conf::types::Strategy;
use crate::traffic::{algorithms::*, decision::*, snapshot::*, strategy::TrafficStrategy};
use once_cell::sync::Lazy;

static FAILOVER: Lazy<Failover> = Lazy::new(|| Failover::default());
static HASH: Lazy<Hash> = Lazy::new(|| Hash::default());
static LEAST_CONNECTIONS: Lazy<LeastConnections> = Lazy::new(|| LeastConnections::default());
static RANDOM: Lazy<Random> = Lazy::new(|| Random::default());
static ROUND_ROBIN: Lazy<RoundRobin> = Lazy::new(|| RoundRobin::default());

#[derive(Debug, Default)]
pub struct TrafficDirector;

impl TrafficDirector {
    pub fn decide(
        &self,
        req: &crate::ctx::RequestCtx,
        snapshot: &TrafficSnapshot,
        service_id: &crate::traffic::types::ServiceId,
    ) -> Result<TrafficDecision, TrafficError> {
        let service = snapshot
            .services
            .get(service_id)
            .ok_or(TrafficError::UnknownService)?;

        // Filter healthy upstreams first
        let healthy: Vec<_> = service
            .upstreams
            .iter()
            .filter(|u| u.health.healthy)
            .cloned()
            .collect();

        if healthy.is_empty() {
            return Err(TrafficError::NoHealthyUpstreams);
        }

        // Delegate to strategy
        let strategy: &dyn TrafficStrategy = match service.strategy {
            Strategy::Failover => &*FAILOVER,
            Strategy::RoundRobin => &*ROUND_ROBIN,
            Strategy::LeastConnections => &*LEAST_CONNECTIONS,
            Strategy::Hash => &*HASH,
            Strategy::Random => &*RANDOM,
        };

        if let Some(decision) = strategy.decide(req, &healthy) {
            return Ok(decision);
        }

        // Hard fallback: first healthy
        Ok(TrafficDecision {
            upstream_id: healthy[0].endpoint.id,
            reason: DecisionReason::ForcedSingle,
            protocol: None,
        })
    }
}

#[derive(Debug)]
pub enum TrafficError {
    UnknownService,
    NoHealthyUpstreams,
}
