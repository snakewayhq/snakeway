use std::time::SystemTime;

#[derive(Clone)]
pub(crate) enum LogEvent {
    Snakeway(SnakewayEvent),
    Generic(GenericEvent),
}

#[derive(Clone)]
pub(crate) struct SnakewayEvent {
    pub(crate) request_id: Option<String>,
    pub(crate) level: String,
    pub(crate) name: String,
    pub(crate) method: Option<String>,
    pub(crate) uri: Option<String>,
    /// status is a string in logs; we parse to i64
    pub(crate) status: Option<i64>,
    pub(crate) ts: Option<SystemTime>,
    pub(crate) identity: Option<IdentitySummary>,
}

#[derive(Clone, Default)]
pub(crate) struct IdentitySummary {
    pub(crate) device: Option<String>,
    pub(crate) bot: Option<bool>,
    pub(crate) asn: Option<usize>,
    pub(crate) aso: Option<String>,
    pub(crate) connection_type: Option<String>,
    pub(crate) country: Option<String>,
}

#[derive(Clone)]
pub(crate) struct GenericEvent {
    pub(crate) level: String,
    pub(crate) message: String,
    pub(crate) target: Option<String>,
}
