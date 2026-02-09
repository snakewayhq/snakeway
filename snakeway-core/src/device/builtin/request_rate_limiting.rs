use crate::conf::types::RequestRateLimitingDeviceConfig;
use crate::ctx::{RequestCtx, ResponseCtx};
use crate::device::core::{Device, DeviceResult};
use pingora_limits::rate::Rate;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use tracing::debug;

#[derive(Clone)]
pub struct RequestRateLimitingDevice {
    rate: Arc<Rate>,
    max_requests_per_second: f64,
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
        Self {
            rate: Arc::new(Rate::new(cfg.reaction_interval)),
            max_requests_per_second: cfg.max_requests_per_second,
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
        // No identity ⇒ no-op
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
