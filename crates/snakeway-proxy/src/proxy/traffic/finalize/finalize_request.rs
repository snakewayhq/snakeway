use crate::proxy::TrafficProxy;
use pingora::Error;
use snakeway_engine::ctx::RequestCtx;

impl TrafficProxy {
    pub(crate) fn finalize_request(&self, ctx: &mut RequestCtx, e: Option<&Error>) {
        // Classify Pingora transport error and set as upstream outcome.
        self.capture_transport_level_failure(ctx, e);

        // Finalize request guard...
        self.finalize_admission_guard(ctx);

        // Record metrics (no-op when OTel is disabled).
        self.record_metrics(ctx);
    }
}
