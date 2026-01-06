use crate::ctx::RequestCtx;
use crate::enrichment::user_agent::ClientIdentity;
use crate::traffic_management::{
    ServiceId, TrafficManager,
    decision::{DecisionReason, TrafficDecision},
    snapshot::UpstreamSnapshot,
    strategy::TrafficStrategy,
};
use ahash::RandomState;
use std::hash::Hash;

#[derive(Debug, Default)]
pub struct StickyHash;

impl StickyHash {
    /// Deterministic, fast hash for routing decisions.
    ///
    /// Fixed seeds:
    /// - Stable across restarts
    /// - Stable across processes
    /// - Not security-sensitive
    fn hash_to_u64<T: Hash>(&self, value: &T) -> u64 {
        static HASHER: RandomState = RandomState::with_seeds(1, 2, 3, 4);
        HASHER.hash_one(value)
    }

    /// Resolve a stable stickiness key for the request.
    ///
    /// Priority:
    /// 1. Explicit header (`x-sticky-key`)
    /// 2. Identity device (if enabled)
    /// 3. Raw peer IP (always exists)
    fn resolve_sticky_key(&self, req: &RequestCtx) -> Option<String> {
        if let Some(v) = req
            .headers
            .get("x-sticky-key")
            .and_then(|h| h.to_str().ok())
            .filter(|v| !v.is_empty())
        {
            return Some(v.to_owned());
        }

        if let Some(identity) = req.extensions.get::<ClientIdentity>() {
            return Some(identity.ip.to_string());
        }

        Some(req.peer_ip.to_string())
    }

    /// Rendezvous hashing: choose the upstream with the highest score.
    fn rendezvous<'a>(
        &self,
        key: &str,
        upstreams: &'a [UpstreamSnapshot],
    ) -> Option<&'a UpstreamSnapshot> {
        upstreams.iter().max_by_key(|u| {
            // Combine sticky key and upstream identity
            self.hash_to_u64(&(key, u.endpoint.id()))
        })
    }
}

impl TrafficStrategy for StickyHash {
    fn decide(
        &self,
        req: &RequestCtx,
        _service_id: &ServiceId,
        healthy: &[UpstreamSnapshot],
        _traffic_manager: &TrafficManager,
    ) -> Option<TrafficDecision> {
        if healthy.is_empty() {
            return None;
        }

        let key = self.resolve_sticky_key(req)?;
        let upstream = self.rendezvous(&key, healthy)?;

        Some(TrafficDecision {
            upstream_id: upstream.endpoint.id(),
            reason: DecisionReason::StickyHash,
            protocol: None,
            cb_started: true,
        })
    }
}
