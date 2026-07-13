use crate::execution::traffic::{ServiceId, TrafficManager};
use crate::runtime::UpstreamId;
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug)]
pub struct AdmissionGuard {
    tm: Arc<TrafficManager>,
    service_id: ServiceId,
    upstream_id: UpstreamId,
    started: Instant,
    finished: bool,
}

impl AdmissionGuard {
    pub fn new(
        tm: Arc<TrafficManager>,
        service_id: ServiceId,
        upstream_id: UpstreamId,
    ) -> Self {
        tm.on_request_start(&service_id, &upstream_id);

        Self {
            tm,
            service_id,
            upstream_id,
            started: Instant::now(),
            finished: false,
        }
    }

    pub fn success(&mut self) {
        self.finish(true);
    }

    pub fn failure(&mut self) {
        self.finish(false);
    }

    fn finish(&mut self, success: bool) {
        if self.finished {
            return;
        }

        // Capture success/failure.
        if success {
            let latency = self.started.elapsed();
            self.tm
                .report_success(&self.service_id, &self.upstream_id, latency);
        } else {
            self.tm.report_failure(&self.service_id, &self.upstream_id);
        }

        self.tm
            .circuit_on_end(&self.service_id, &self.upstream_id, true, success);

        self.tm.on_request_end(&self.service_id, &self.upstream_id);

        self.finished = true;
    }
}

impl Drop for AdmissionGuard {
    fn drop(&mut self) {
        if !self.finished {
            // This covers a lot of potential faults...
            // - upstream crash
            // - canceled future
            // - panic
            // - early return
            tracing::warn!(
                service = %self.service_id,
                upstream = ?self.upstream_id,
                "request guard dropped without explicit completion"
            );
            self.finish(false);
        }
    }
}
