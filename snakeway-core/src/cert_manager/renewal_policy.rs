use std::time::Duration;

#[derive(Clone)]
pub struct RenewalPolicy {
    pub reconcile_interval: Duration,
    pub order_poll_interval: Duration,
    pub renew_within: Duration,
}

impl RenewalPolicy {
    pub fn new(renew_within_days: u64) -> Self {
        Self {
            reconcile_interval: Duration::from_hours(24),
            order_poll_interval: Duration::from_secs(5),
            renew_within: Duration::from_secs(60 * 60 * 24 * renew_within_days),
        }
    }
}
