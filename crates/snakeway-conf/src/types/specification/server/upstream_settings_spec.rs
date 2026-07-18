use crate::types::{HclInt, UpstreamSourceAddressesSpec};
use confval::prelude::Located;
use serde::Serialize;

#[derive(Debug, Serialize, Default, confval::Spec)]
pub struct UpstreamSettingsSpec {
    /// Idle upstream connections kept warm per worker thread.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_pool_size: Option<Located<HclInt>>,

    /// Connect timeout (seconds) for TCP plus TLS. Omit to disable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_timeout_seconds: Option<Located<HclInt>>,

    /// Per-read (idle) timeout (seconds) for upstream responses. Omit to disable.
    /// Not applied to websocket upgrades.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_timeout_seconds: Option<Located<HclInt>>,

    /// Local source addresses for outbound upstream connections.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[confval(nested)]
    pub source_addresses: Option<Located<UpstreamSourceAddressesSpec>>,
}
