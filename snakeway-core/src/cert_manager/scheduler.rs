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

    pub fn renewal_threshold(&self) -> Duration {
        Duration::from_secs(60 * 60 * 24 * 14) // 14 days
    }
}
