use crate::execution::ctx::RequestCtx;
use crate::execution::device::builtin::request_rate_limiting::RequestRateLimitingDevice;
use crate::execution::device::core::{Device, DeviceResult};
use crate::execution::enrichment::user_agent::ClientIdentity;
use snakeway_conf::types::RequestRateLimitingDeviceConfig;
use std::net::{IpAddr, Ipv4Addr};
use std::time::Duration;
use tokio::time::sleep;

//-----------------------------------------------------------------------------
// Helpers
//-----------------------------------------------------------------------------

fn ctx_with_identity(identity: ClientIdentity) -> RequestCtx {
    let mut ctx = RequestCtx::empty();
    ctx.extensions.insert(identity);
    ctx
}

fn identity(ip: IpAddr) -> ClientIdentity {
    ClientIdentity {
        ip,
        proxy_chain: vec![],
        is_forwarded: false,
        is_trusted: true,
        geo: None,
        ua: None,
    }
}

fn device() -> RequestRateLimitingDevice {
    let cfg = RequestRateLimitingDeviceConfig {
        enable: true,
        reaction_interval: Duration::from_secs(1),
        max_requests_per_second: 1.0,
    };
    RequestRateLimitingDevice::from(cfg)
}

//-----------------------------------------------------------------------------
// Identity handling
//-----------------------------------------------------------------------------

#[test]
fn no_identity_is_noop() {
    // Arrange
    let device = device();
    let mut ctx = RequestCtx::empty();

    // Act
    let result = device.on_request(&mut ctx);

    // Assert
    matches!(result, DeviceResult::Continue);
}

//-----------------------------------------------------------------------------
// Basic allow behavior
//-----------------------------------------------------------------------------

#[test]
fn allows_single_request_under_limit() {
    // Arrange
    let device = device();
    let mut ctx = ctx_with_identity(identity(IpAddr::V4(Ipv4Addr::LOCALHOST)));

    // Act
    let result = device.on_request(&mut ctx);

    // Assert
    matches!(result, DeviceResult::Continue);
}

//-----------------------------------------------------------------------------
// Soft rejection behavior (estimator-based)
//-----------------------------------------------------------------------------

#[tokio::test]
async fn sustained_requests_eventually_reject() {
    // Arrange
    let device = device();
    let mut ctx = ctx_with_identity(identity(IpAddr::V4(Ipv4Addr::LOCALHOST)));

    // Act: apply pressure across time
    let start = std::time::Instant::now();
    let mut saw_reject = false;

    while start.elapsed() < Duration::from_secs(2) {
        match device.on_request(&mut ctx) {
            DeviceResult::Respond(_) => {
                saw_reject = true;
                break;
            }
            DeviceResult::Continue => {}
            DeviceResult::Error(_) => {}
        }

        // Yield to allow interval rollover
        tokio::task::yield_now().await;
    }

    // Assert
    assert!(
        saw_reject,
        "expected request rate limiter to eventually reject under sustained pressure"
    );
}

//-----------------------------------------------------------------------------
// Per-identity isolation
//-----------------------------------------------------------------------------

#[tokio::test]
async fn rate_limiting_is_isolated_per_identity() {
    // Arrange
    let device = device();

    let mut ctx_a = ctx_with_identity(identity(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
    let mut ctx_b = ctx_with_identity(identity(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2))));

    // Saturate identity A
    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_secs(2) {
        let _ = device.on_request(&mut ctx_a);
        tokio::task::yield_now().await;
    }

    // Act
    let result_b = device.on_request(&mut ctx_b);

    // Assert
    matches!(result_b, DeviceResult::Continue);
}

//-----------------------------------------------------------------------------
// Recovery / liveness
//-----------------------------------------------------------------------------

#[tokio::test]
async fn limiter_does_not_permanently_reject_after_pressure_stops() {
    // Arrange
    let device = device();
    let mut ctx = ctx_with_identity(identity(IpAddr::V4(Ipv4Addr::LOCALHOST)));

    // Apply sustained pressure
    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_secs(2) {
        let _ = device.on_request(&mut ctx);
        tokio::task::yield_now().await;
    }

    // Stop traffic completely
    sleep(Duration::from_secs(2)).await;

    // Act: try repeatedly until allowed
    let mut saw_allow = false;
    for _ in 0..50 {
        if matches!(device.on_request(&mut ctx), DeviceResult::Continue) {
            saw_allow = true;
            break;
        }
        sleep(Duration::from_millis(20)).await;
    }

    // Assert
    assert!(
        saw_allow,
        "expected request rate limiter to eventually allow after traffic stops"
    );
}
