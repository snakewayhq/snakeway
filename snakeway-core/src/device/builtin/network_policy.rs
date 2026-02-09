use crate::conf::types::{NetworkPolicyDeviceConfig, OnInvalidForwardedConfig};
use crate::ctx::{RequestCtx, ResponseCtx};
use crate::device::core::{Device, DeviceResult};
use crate::net::CidrCollection;
use tracing::debug;

#[derive(Debug)]
pub struct NetworkPolicyDevice {
    pub(crate) cidr_allow: CidrCollection,
    pub(crate) allow_forwarded: bool,
    pub(crate) on_invalid_forwarded: OnInvalidForwarded,
}

#[derive(Debug, Clone, Copy)]
pub enum OnInvalidForwarded {
    Deny,
    Ignore,
}

impl From<NetworkPolicyDeviceConfig> for NetworkPolicyDevice {
    fn from(cfg: NetworkPolicyDeviceConfig) -> Self {
        Self {
            cidr_allow: cfg.cidr_allow.into(),
            allow_forwarded: cfg.forwarding.allow,
            on_invalid_forwarded: cfg.forwarding.on_invalid.into(),
        }
    }
}

impl From<OnInvalidForwardedConfig> for OnInvalidForwarded {
    fn from(cfg: OnInvalidForwardedConfig) -> Self {
        match cfg {
            OnInvalidForwardedConfig::Deny => OnInvalidForwarded::Deny,
            OnInvalidForwardedConfig::Ignore => OnInvalidForwarded::Ignore,
        }
    }
}

impl NetworkPolicyDevice {
    #[inline]
    fn deny(&self, ctx: &RequestCtx, reason: &'static str) -> DeviceResult {
        debug!(
            request_id = ctx.request_id(),
            reason, "network policy denied request"
        );
        DeviceResult::Respond(ResponseCtx::forbidden(ctx.request_id()))
    }
}

impl Device for NetworkPolicyDevice {
    fn name(&self) -> &str {
        "Network Policy"
    }

    fn on_request(&self, ctx: &mut RequestCtx) -> DeviceResult {
        // No identity, then short-circuit the device.
        // However, this should never happen, as the identity device must run before this device.
        // A config validation error would have been caught earlier.
        let identity = match ctx.identity() {
            Some(id) => id,
            None => return DeviceResult::Continue,
        };

        // Base admission, the resolved client IP must be allowed, but it is not in the allowlist.
        if !self.cidr_allow.contains(identity.ip) {
            return self.deny(ctx, "client ip not in allowlist");
        }

        // Forwarded handling is strictly more restrictive.
        if identity.is_forwarded {
            if !self.allow_forwarded {
                return self.deny(ctx, "forwarded request not allowed");
            }

            if !identity.is_trusted {
                match self.on_invalid_forwarded {
                    OnInvalidForwarded::Deny => {
                        return self.deny(ctx, "invalid forwarded identity");
                    }
                    OnInvalidForwarded::Ignore => {
                        debug!(
                            request_id = ctx.request_id(),
                            ip = %identity.ip,
                            "invalid forwarded identity ignored; using peer ip"
                        );
                    }
                }
            }
        }

        DeviceResult::Continue
    }
}
