use crate::types::HclInt;
use confval::diagnostic::Report;
use confval::prelude::{Located, Validate};
use confval::{RangeConstraint, range_constraint};
use serde::Serialize;
use std::net::{Ipv4Addr, Ipv6Addr};

range_constraint!(CONNECTION_POOL_SIZE, i64, min: 1, max: 65535);
range_constraint!(TIMEOUT_SECONDS, i64, min: 1, max: 3600, units: "seconds");

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

impl Validate for UpstreamSettingsSpec {
    fn validate(&self, report: &mut Report) {
        if let Some(pool_size) = &self.connection_pool_size {
            CONNECTION_POOL_SIZE.check_located(pool_size, "connection_pool_size", report);
        }
        if let Some(timeout) = &self.connection_timeout_seconds {
            TIMEOUT_SECONDS.check_located(timeout, "connection_timeout_seconds", report);
        }
        if let Some(timeout) = &self.read_timeout_seconds {
            TIMEOUT_SECONDS.check_located(timeout, "read_timeout_seconds", report);
        }
        if let Some(source_addresses) = &self.source_addresses {
            source_addresses.validate(report);
        }
    }
}

#[derive(Debug, Serialize, Default, confval::Spec)]
pub struct UpstreamSourceAddressesSpec {
    #[confval(default)]
    pub ipv4: Vec<Located<String>>,
    #[confval(default)]
    pub ipv6: Vec<Located<String>>,
}

impl Validate for UpstreamSourceAddressesSpec {
    fn validate(&self, report: &mut Report) {
        for addr in &self.ipv4 {
            if addr.value.parse::<Ipv4Addr>().is_err() {
                report
                    .error(format!(
                        "invalid upstream.source_addresses.ipv4 entry: \"{}\" is not a valid IPv4 address",
                        addr.value
                    ))
                    .at(addr.span)
                    .emit();
            }
        }
        for addr in &self.ipv6 {
            if addr.value.parse::<Ipv6Addr>().is_err() {
                report
                    .error(format!(
                        "invalid upstream.source_addresses.ipv6 entry: \"{}\" is not a valid IPv6 address",
                        addr.value
                    ))
                    .at(addr.span)
                    .emit();
            }
        }
    }
}
