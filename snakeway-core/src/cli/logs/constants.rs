use std::time::Duration;

pub(crate) const WINDOW: Duration = Duration::from_secs(10);
pub(crate) const RENDER_TICK: Duration = Duration::from_secs(1);
pub(crate) const IN_FLIGHT_TTL: Duration = Duration::from_secs(60);
pub(crate) const LOOP_IDLE_SLEEP: Duration = Duration::from_millis(25);
