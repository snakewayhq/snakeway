use crate::execution::ctx::{RequestCtx, ResponseCtx};
use crate::execution::device::core::{Device, DeviceResult};
use crate::execution::route::path_matches_prefix;
use pingora_limits::rate::Rate;
use smallvec::SmallVec;
use snakeway_conf::types::RequestRateLimitingDeviceConfig;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use tracing::debug;

#[derive(Clone)]
pub(crate) struct RequestRateLimitingDevice {
    rate: Arc<Rate>,
    max_requests_per_second: f64,
    paths: SmallVec<[String; 4]>,
}

impl Debug for RequestRateLimitingDevice {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RequestRateLimitingDevice")
            .field("max_requests_per_second", &self.max_requests_per_second)
            .finish()
    }
}

impl From<RequestRateLimitingDeviceConfig> for RequestRateLimitingDevice {
    fn from(cfg: RequestRateLimitingDeviceConfig) -> Self {
        let mut paths = cfg.paths;
        paths.sort_by_key(|p| std::cmp::Reverse(p.len()));
        Self {
            rate: Arc::new(Rate::new(cfg.reaction_interval)),
            max_requests_per_second: cfg.max_requests_per_second,
            paths,
        }
    }
}

impl RequestRateLimitingDevice {
    #[inline]
    fn deny(&self, ctx: &RequestCtx, reason: &'static str) -> DeviceResult {
        debug!(
            request_id = ctx.request_id(),
            reason, "request rate limit exceeded"
        );

        DeviceResult::Respond(ResponseCtx::too_many_requests(ctx.request_id()))
    }
}

impl Device for RequestRateLimitingDevice {
    fn name(&self) -> &str {
        "Request Rate Limit"
    }

    fn on_request(&self, ctx: &mut RequestCtx) -> DeviceResult {
        // Skip if the request path does not match any configured path scope.
        if !self.paths.is_empty()
            && !self
                .paths
                .iter()
                .any(|p| path_matches_prefix(p, ctx.canonical_path()))
        {
            return DeviceResult::Continue;
        }

        // No identity, then short-circuit the device.
        // However, this should never happen, as the identity device must run before this device.
        // A config validation error would have been caught earlier.
        let identity = match ctx.identity() {
            Some(id) => id,
            None => return DeviceResult::Continue,
        };

        let key = identity.ip;

        // Observe this request
        self.rate.observe(&key, 1);

        // Check estimated rate (requests/sec)
        let current_rate = self.rate.rate(&key);

        if current_rate > self.max_requests_per_second {
            return self.deny(ctx, "request rate exceeded");
        }

        DeviceResult::Continue
    }
}
