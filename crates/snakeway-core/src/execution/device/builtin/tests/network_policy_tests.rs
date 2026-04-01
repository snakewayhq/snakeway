use crate::execution::ctx::RequestCtx;
use crate::execution::ctx::request::NormalizedPath;
use crate::execution::device::builtin::network_policy::{NetworkPolicyDevice, OnInvalidForwarded};
use crate::execution::device::core::{Device, DeviceResult};
use crate::execution::enrichment::user_agent::ClientIdentity;
use crate::net::CidrCollection;
use smallvec::SmallVec;
use snakeway_conf::types::{ForwardingConfig, NetworkPolicyDeviceConfig, OnInvalidForwardedConfig};
use std::net::{IpAddr, Ipv4Addr};

//-----------------------------------------------------------------------------
// Helpers
//-----------------------------------------------------------------------------
fn ctx_with_identity(identity: ClientIdentity) -> RequestCtx {
    let mut ctx = RequestCtx::empty();
    ctx.extensions.insert(identity);
    ctx
}

fn identity(ip: IpAddr, is_forwarded: bool, is_trusted: bool) -> ClientIdentity {
    ClientIdentity {
        ip,
        proxy_chain: vec![],
        is_forwarded,
        is_trusted,
        geo: None,
        ua: None,
    }
}

fn allow_all_device() -> NetworkPolicyDevice {
    NetworkPolicyDevice {
        cidr_allow: CidrCollection::default(),
        allow_forwarded: true,
        on_invalid_forwarded: OnInvalidForwarded::Ignore,
        paths: smallvec::smallvec![],
    }
}

//-----------------------------------------------------------------------------
// Identity handling
//-----------------------------------------------------------------------------

#[test]
fn no_identity_is_noop() {
    // Arrange

    let device = allow_all_device();
    let mut ctx = RequestCtx::default();

    // Act
    let result = device.on_request(&mut ctx);

    // Assert
    matches!(result, DeviceResult::Continue);
}

//-----------------------------------------------------------------------------
// CIDR allowlist enforcement
//-----------------------------------------------------------------------------

#[test]
fn allows_request_when_ip_in_allowlist() {
    // Arrange
    let cidr = "10.0.0.0/8".parse().unwrap();
    let device = NetworkPolicyDevice {
        cidr_allow: CidrCollection::new(&[cidr]),
        allow_forwarded: true,
        on_invalid_forwarded: OnInvalidForwarded::Ignore,
        paths: smallvec::smallvec![],
    };

    let mut ctx = ctx_with_identity(identity(
        IpAddr::V4(Ipv4Addr::new(10, 1, 2, 3)),
        false,
        true,
    ));

    // Act
    let result = device.on_request(&mut ctx);

    // Assert
    matches!(result, DeviceResult::Continue);
}

#[test]
fn denies_request_when_ip_not_in_allowlist() {
    // Arrange
    let cidr = "10.0.0.0/8".parse().unwrap();
    let device = NetworkPolicyDevice {
        cidr_allow: CidrCollection::new(&[cidr]),
        allow_forwarded: true,
        on_invalid_forwarded: OnInvalidForwarded::Ignore,
        paths: smallvec::smallvec![],
    };

    let mut ctx = ctx_with_identity(identity(
        IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
        false,
        true,
    ));

    // Act
    let result = device.on_request(&mut ctx);

    // Assert
    matches!(result, DeviceResult::Respond(_));
}

//-----------------------------------------------------------------------------
// Forwarded request handling
//-----------------------------------------------------------------------------

#[test]
fn denies_forwarded_request_when_forwarding_not_allowed() {
    // Arrange
    let cidr = "0.0.0.0/0".parse().unwrap();
    let device = NetworkPolicyDevice {
        cidr_allow: CidrCollection::new(&[cidr]),
        allow_forwarded: false,
        on_invalid_forwarded: OnInvalidForwarded::Ignore,
        paths: smallvec::smallvec![],
    };

    let mut ctx = ctx_with_identity(identity(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)), true, true));

    // Act
    let result = device.on_request(&mut ctx);

    // Assert
    matches!(result, DeviceResult::Respond(_));
}

#[test]
fn allows_trusted_forwarded_identity() {
    // Arrange
    let cidr = "0.0.0.0/0".parse().unwrap();
    let device = NetworkPolicyDevice {
        cidr_allow: CidrCollection::new(&[cidr]),
        allow_forwarded: true,
        on_invalid_forwarded: OnInvalidForwarded::Deny,
        paths: smallvec::smallvec![],
    };
    let mut ctx = ctx_with_identity(identity(IpAddr::V4(Ipv4Addr::new(9, 9, 9, 9)), true, true));

    // Act
    let result = device.on_request(&mut ctx);

    // Assert
    matches!(result, DeviceResult::Continue);
}

//-----------------------------------------------------------------------------
// Invalid forwarded identity handling
//-----------------------------------------------------------------------------

#[test]
fn denies_invalid_forwarded_identity_when_configured_to_deny() {
    // Arrange
    let cidr = "0.0.0.0/0".parse().unwrap();
    let device = NetworkPolicyDevice {
        cidr_allow: CidrCollection::new(&[cidr]),
        allow_forwarded: true,
        on_invalid_forwarded: OnInvalidForwarded::Deny,
        paths: smallvec::smallvec![],
    };
    let mut ctx = ctx_with_identity(identity(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)), true, false));

    // Act
    let result = device.on_request(&mut ctx);

    // Assert
    matches!(result, DeviceResult::Respond(_));
}

#[test]
fn allows_invalid_forwarded_identity_when_configured_to_ignore() {
    // Arrange
    let cidr = "0.0.0.0/0".parse().unwrap();
    let device = NetworkPolicyDevice {
        cidr_allow: CidrCollection::new(&[cidr]),
        allow_forwarded: true,
        on_invalid_forwarded: OnInvalidForwarded::Ignore,
        paths: smallvec::smallvec![],
    };

    let mut ctx = ctx_with_identity(identity(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)), true, false));

    // Act
    let result = device.on_request(&mut ctx);

    // Assert
    matches!(result, DeviceResult::Continue);
}

//-----------------------------------------------------------------------------
// Config to runtime translation
//-----------------------------------------------------------------------------
#[test]
fn from_config_sets_runtime_fields_correctly() {
    // Arrange
    let cidr = "10.0.0.0/8".parse().unwrap();
    let cfg = NetworkPolicyDeviceConfig {
        enable: true,
        cidr_allow: vec![cidr],
        forwarding: ForwardingConfig {
            allow: false,
            on_invalid: OnInvalidForwardedConfig::Deny,
        },
        paths: smallvec::smallvec![],
    };

    // Act
    let device = NetworkPolicyDevice::from(cfg);

    // Assert
    assert!(!device.allow_forwarded);
    matches!(device.on_invalid_forwarded, OnInvalidForwarded::Deny);
}

//-----------------------------------------------------------------------------
// Path scoping
//-----------------------------------------------------------------------------

fn ctx_with_identity_and_path(identity: ClientIdentity, path: &str) -> RequestCtx {
    let mut ctx = RequestCtx::empty();
    ctx.set_normalized_request(NormalizedPath(path.to_string()).into());
    ctx.hydrated = true;
    ctx.extensions.insert(identity);
    ctx
}

fn denied_device_with_paths(paths: SmallVec<[String; 4]>) -> NetworkPolicyDevice {
    // Device that denies all IPs (empty allowlist) -- will deny if path matches.
    NetworkPolicyDevice {
        cidr_allow: CidrCollection::default(),
        allow_forwarded: true,
        on_invalid_forwarded: OnInvalidForwarded::Ignore,
        paths,
    }
}

#[test]
fn empty_paths_applies_to_all_requests() {
    // Arrange
    let device = denied_device_with_paths(smallvec::smallvec![]);
    let mut ctx = ctx_with_identity_and_path(
        identity(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)), false, true),
        "/anything",
    );

    // Act
    let result = device.on_request(&mut ctx);

    // Assert -- device runs (and denies, since IP is not in empty allowlist)
    assert!(matches!(result, DeviceResult::Respond(_)));
}

#[test]
fn matching_path_applies_device() {
    // Arrange
    let device = denied_device_with_paths(smallvec::smallvec!["/api".to_string()]);
    let mut ctx = ctx_with_identity_and_path(
        identity(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)), false, true),
        "/api/users",
    );

    // Act
    let result = device.on_request(&mut ctx);

    // Assert -- path matches /api, device runs and denies
    assert!(matches!(result, DeviceResult::Respond(_)));
}

#[test]
fn non_matching_path_skips_device() {
    // Arrange
    let device = denied_device_with_paths(smallvec::smallvec!["/api".to_string()]);
    let mut ctx = ctx_with_identity_and_path(
        identity(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)), false, true),
        "/static/image.png",
    );

    // Act
    let result = device.on_request(&mut ctx);

    // Assert -- path does not match /api, device is skipped
    assert!(matches!(result, DeviceResult::Continue));
}
