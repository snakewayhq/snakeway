use crate::proxy::TrafficProxy;
use snakeway_engine::ctx::RequestCtx;
use snakeway_engine::traffic::UpstreamOutcome;

impl TrafficProxy {
    /// Finalizes the request guard by reporting success or failure to the traffic manager.
    ///
    /// This method determines the outcome of the request based on the upstream response
    /// and circuit breaker configuration. It marks the request as successful or failed,
    /// which updates the circuit breaker state for the selected upstream.
    ///
    /// Success criteria:
    /// - No transport error occurred
    /// - HTTP status < 500 (if count_http_5xx_as_failure is true)
    /// - Any status code (if count_http_5xx_as_failure is false)
    ///
    /// This is called from the logging hook to ensure it runs after all other processing.
    pub(crate) fn finalize_admission_guard(&self, ctx: &mut RequestCtx) {
        let (service_id, _) = match ctx.selected_upstream.as_ref() {
            Some(v) => v,
            None => return,
        };

        let guard = match ctx.admission_guard.as_mut() {
            Some(g) => g,
            None => return,
        };

        let success = match ctx.upstream_outcome {
            Some(UpstreamOutcome::Transport(failure)) => {
                tracing::debug!(
                    service = %service_id,
                    failure = ?failure,
                    "upstream transport failure"
                );
                false
            }

            Some(UpstreamOutcome::HttpStatus(code)) => {
                let count_5xx = self
                    .proxy_ctx
                    .traffic_manager
                    .count_http_5xx_as_failure(service_id)
                    .unwrap_or(true);

                if count_5xx { code < 500 } else { true }
            }

            Some(UpstreamOutcome::Success) => true,

            None => true,
        };

        if success {
            guard.success();
        } else {
            guard.failure();
        }
    }
}
