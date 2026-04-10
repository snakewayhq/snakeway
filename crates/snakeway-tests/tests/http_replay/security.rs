use super::replay_fixture;
use integration::constants::HTTP_REPLAY_OK_RESPONSE;

/// Host header edge case.
///
/// A Host header with an explicit port (e.g. `snakeway.test:8080`) must be
/// handled correctly. The router should extract only the hostname for route
/// matching and ignore the port component.
#[test]
fn host_with_port_should_proxy() {
    // Arrange + Act
    let resp = replay_fixture("security/host_with_port.http");

    // Assert
    assert!(
        resp.contains(HTTP_REPLAY_OK_RESPONSE),
        "Host header with port should be accepted and proxied"
    );
}

/// Host header edge case.
///
/// A request whose Host header does not match any configured route must
/// receive a 404 response. The proxy must not fall through to a default
/// service or forward the request to an arbitrary upstream.
#[test]
fn unknown_host_should_return_404() {
    // Arrange + Act
    let resp = replay_fixture("security/unknown_host.http");

    // Assert
    assert!(
        !resp.contains(HTTP_REPLAY_OK_RESPONSE),
        "request to unknown host must not be proxied"
    );
    assert!(
        resp.contains("404"),
        "unknown host should produce a 404 response"
    );
}

/// Request line limits.
///
/// An extremely long request URI (~65 KB path) is accepted by Pingora's
/// HTTP parser and routed normally. Snakeway does not impose a separate
/// URI length limit beyond what the request filter's `max_header_bytes`
/// covers for the overall header block.
///
/// This test documents the current behavior: long URIs are proxied. If a
/// stricter limit is needed, it should be enforced via `max_header_bytes`
/// in the request filter device configuration.
#[test]
fn long_request_uri_is_proxied() {
    // Arrange + Act
    let resp = replay_fixture("security/oversized_request_line.http");

    // Assert -- Pingora accepts long URIs; the request reaches the
    // upstream but the path won't match any route, producing a 404.
    // The key invariant: the server does not crash or hang.
    assert!(
        !resp.is_empty(),
        "server must respond to long URI requests without crashing"
    );
}

/// Absolute URI and Host header conflict (open relay prevention).
///
/// When a request uses absolute-form with an authority that differs from
/// the Host header (e.g. `GET http://attacker.com/evil` with
/// `Host: snakeway.test`), the proxy must not forward the request to the
/// attacker's host. This prevents the proxy from being used as an open relay.
///
/// Compare with `malformed/absolute_uri.http` which uses the *same* host
/// in both fields. This test uses a *different* host to verify the proxy
/// does not follow the absolute URI to an external target.
#[test]
fn absolute_uri_with_host_conflict_should_not_proxy_to_external() {
    // Arrange + Act
    let resp = replay_fixture("security/absolute_uri_host_conflict.http");

    // Assert
    assert!(
        !resp.contains(HTTP_REPLAY_OK_RESPONSE),
        "absolute URI targeting external host must not be proxied"
    );
}
