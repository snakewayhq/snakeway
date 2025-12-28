use crate::ctx::RequestCtx;
use crate::traffic::{decision::*, snapshot::*, strategy::TrafficStrategy};
use ahash::RandomState;
use std::hash::{BuildHasher, Hash, Hasher};

#[derive(Debug, Default)]
pub struct StickyHash {}

impl StickyHash {
    pub fn hash_to_u64(&self, s: &str) -> u64 {
        static HASHER: RandomState = RandomState::with_seeds(1, 2, 3, 4);

        let mut hasher = HASHER.build_hasher();
        s.hash(&mut hasher);
        hasher.finish()
    }
}
impl TrafficStrategy for StickyHash {
    fn decide(&self, req: &RequestCtx, healthy: &[UpstreamSnapshot]) -> Option<TrafficDecision> {
        if healthy.is_empty() {
            return None;
        }

        let value = match req.headers.get("x-user-id").and_then(|h| h.to_str().ok()) {
            Some(v) => v,
            None => {
                // No stickiness signal. Let caller fall back.
                return None;
            }
        };
        let hashed_value = self.hash_to_u64(value);
        let idx = (hashed_value as usize) % healthy.len();
        let upstream_snapshot = &healthy[idx];

        Some(TrafficDecision {
            upstream_id: upstream_snapshot.endpoint.id,
            reason: DecisionReason::StickyHash,
            protocol: None,
        })
    }
}
