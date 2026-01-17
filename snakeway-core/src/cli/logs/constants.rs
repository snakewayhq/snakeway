use std::time::Duration;

pub const WINDOW: Duration = Duration::from_secs(10);
pub const RENDER_TICK: Duration = Duration::from_secs(1);
pub const IN_FLIGHT_TTL: Duration = Duration::from_secs(60);
pub const LOOP_IDLE_SLEEP: Duration = Duration::from_millis(25);
