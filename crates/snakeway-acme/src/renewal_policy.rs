use std::time::Duration;

#[derive(Clone)]
pub(crate) struct RenewalPolicy {
    pub(crate) reconcile_interval: Duration,
    pub(crate) order_poll_interval: Duration,
    pub(crate) renew_within: Duration,
}

impl RenewalPolicy {
    pub(crate) fn new(renew_within_days: u64) -> Self {
        Self {
            reconcile_interval: Duration::from_hours(24),
            order_poll_interval: Duration::from_secs(5),
            renew_within: Duration::from_secs(60 * 60 * 24 * renew_within_days),
        }
    }
}
