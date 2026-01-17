use std::time::SystemTime;

#[derive(Clone)]
pub enum LogEvent {
    Snakeway(SnakewayEvent),
    Generic(GenericEvent),
}

#[derive(Clone)]
pub struct SnakewayEvent {
    pub request_id: Option<String>,
    pub level: String,
    pub name: String,
    pub method: Option<String>,
    pub uri: Option<String>,
    /// status is a string in logs; we parse to i64
    pub status: Option<i64>,
    pub ts: Option<SystemTime>,
    pub identity: Option<IdentitySummary>,
}

#[derive(Clone, Default)]
pub struct IdentitySummary {
    pub device: Option<String>,
    pub bot: Option<bool>,
}

#[derive(Clone)]
pub struct GenericEvent {
    pub level: String,
    pub message: String,
    pub target: Option<String>,
}
