use std::time::Duration;

#[derive(Clone)]
pub struct Scheduler {
    tick: Duration,
}

impl Default for Scheduler {
    fn default() -> Self {
        Self {
            tick: Duration::from_secs(30),
        }
    }
}

impl Scheduler {
    pub fn tick_interval(&self) -> Duration {
        self.tick
    }

    /// Hardcoding a sane window for now: renew when < 30 days left.
    pub fn renew_within(&self) -> Duration {
        Duration::from_secs(60 * 60 * 24 * 30)
    }
}
