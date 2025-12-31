use crate::server::UpstreamId;
use crate::traffic::{ServiceId, TrafficManager};
use std::sync::Arc;

#[derive(Debug)]
pub struct RequestGuard {
    tm: Arc<TrafficManager>,
    service_id: ServiceId,
    upstream_id: UpstreamId,
    finished: bool,
}

impl RequestGuard {
    pub fn new(tm: Arc<TrafficManager>, service_id: ServiceId, upstream_id: UpstreamId) -> Self {
        tm.on_request_start(&service_id, &upstream_id);

        Self {
            tm,
            service_id,
            upstream_id,
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

        if success {
            self.tm.report_success(&self.service_id, &self.upstream_id);
        } else {
            self.tm.report_failure(&self.service_id, &self.upstream_id);
        }

        self.tm
            .circuit_on_end(&self.service_id, &self.upstream_id, true, success);

        self.tm.on_request_end(&self.service_id, &self.upstream_id);

        self.finished = true;
    }
}

impl Drop for RequestGuard {
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
