use crate::proxy::TrafficProxy;
use pingora::{BError, Custom, Error};
use snakeway_engine::ctx::RequestCtx;
use snakeway_engine::runtime::RuntimeState;
use snakeway_engine::traffic::{SelectedUpstream, ServiceId};

impl TrafficProxy {
    /// Select an upstream for the given request.
    pub(crate) fn select_upstream<'a>(
        &self,
        ctx: &RequestCtx,
        state: &'a RuntimeState,
        service_id: &ServiceId,
        service_name: &str,
    ) -> std::result::Result<SelectedUpstream<'a>, BError> {
        // Get a snapshot (cheap, lock-free)
        let snapshot = self.proxy_ctx.traffic_manager.snapshot();

        // Ask the director for a decision.
        let decision = self
            .traffic_director
            .decide(ctx, &snapshot, service_id, &self.proxy_ctx.traffic_manager)
            .map_err(|e| {
                tracing::error!(error = ?e, "traffic decision failed");
                Error::new(Custom("traffic decision failed"))
            })?;

        tracing::info!("decision reason: {}", decision.reason);

        // Grab the service by name.
        let service = state
            .services
            .get(service_name)
            .ok_or_else(|| Error::new(Custom("unknown service")))?;

        // Get the upstream based on the decision from the Traffic Director.
        let upstream = service
            .upstreams
            .iter()
            .find(|u| u.id() == decision.upstream_id)
            .ok_or_else(|| Error::new(Custom("selected upstream not found")))?;

        Ok(SelectedUpstream {
            upstream,
            cb_started: decision.cb_started,
        })
    }
}
