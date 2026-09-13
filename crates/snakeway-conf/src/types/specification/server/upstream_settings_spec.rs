use confval::diagnostic::Report;
use confval::prelude::{Ipv4, Ipv6, Located, Validate, range_constraint};
use serde::Serialize;

range_constraint!(CONNECTION_POOL_SIZE, i64, min: 1, max: 65535);
range_constraint!(TIMEOUT_SECONDS, i64, min: 1, max: 3600, units: "seconds");

#[derive(Debug, Serialize, Default, confval::Spec)]
pub struct UpstreamSettingsSpec {
    /// Idle upstream connections kept warm per worker thread.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[confval(range = CONNECTION_POOL_SIZE)]
    pub connection_pool_size: Option<Located<i64>>,

    /// Connect timeout (seconds) for TCP plus TLS. Omit to disable.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[confval(range = TIMEOUT_SECONDS)]
    pub connection_timeout_seconds: Option<Located<i64>>,

    /// Per-read (idle) timeout (seconds) for upstream responses. Omit to disable.
    /// Not applied to websocket upgrades.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[confval(range = TIMEOUT_SECONDS)]
    pub read_timeout_seconds: Option<Located<i64>>,

    /// Local source addresses for outbound upstream connections.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[confval(nested)]
    pub source_addresses: Option<Located<UpstreamSourceAddressesSpec>>,
}

impl Validate for UpstreamSettingsSpec {
    fn validate(&self, _report: &mut Report) {}
}

#[derive(Debug, Serialize, Default, confval::Spec)]
pub struct UpstreamSourceAddressesSpec {
    #[confval(default, format = Ipv4)]
    pub ipv4: Vec<Located<String>>,
    #[confval(default, format = Ipv6)]
    pub ipv6: Vec<Located<String>>,
}

impl Validate for UpstreamSourceAddressesSpec {
    fn validate(&self, _report: &mut Report) {}
}
