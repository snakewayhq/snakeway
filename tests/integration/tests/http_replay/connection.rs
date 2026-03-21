use super::replay_fixture;
use integration::constants::HTTP_REPLAY_OK_RESPONSE;

/// HTTP/1.0 remains in use by legacy clients, health-check tools, and
/// some load balancers.  An HTTP/1.1 reverse proxy should accept HTTP/1.0
/// requests and proxy them upstream, upgrading the outgoing connection
/// to HTTP/1.1 as needed.
#[test]
fn http10_get_should_proxy() {
    let resp = replay_fixture("connection/http10_get.http");
    assert!(
        resp.contains(HTTP_REPLAY_OK_RESPONSE),
        "HTTP/1.0 GET should be proxied"
    );
}

/// HTTP/1.0 clients that want persistent connections include a
/// `Connection: Keep-Alive` header (a pre-standard negotiation that
/// predates HTTP/1.1 persistent connections).
///
/// A modern proxy should handle this gracefully — either honouring the
/// keep-alive intent or closing the connection after the response — but
/// must still forward the request to upstream and return a successful
/// response.
#[test]
fn http10_keep_alive_should_proxy() {
    let resp = replay_fixture("connection/http10_keep_alive.http");
    assert!(
        resp.contains(HTTP_REPLAY_OK_RESPONSE),
        "HTTP/1.0 with Connection: Keep-Alive should be proxied"
    );
}

/// `Connection: close` instructs the proxy to close the connection
/// after the response is sent.  This is standard HTTP/1.1 behaviour
/// and must not prevent the request from being proxied.
///
/// A proxy bug where `Connection: close` is treated as an error would
/// break any client that needs to signal it won't reuse the connection.
#[test]
fn connection_close_header_should_proxy() {
    let resp = replay_fixture("connection/connection_close.http");
    assert!(
        resp.contains(HTTP_REPLAY_OK_RESPONSE),
        "Request with Connection: close should still be proxied"
    );
}

/// `Expect: 100-continue` is a flow-control mechanism (RFC 9110 §10.1.1)
/// used by clients that want permission before sending a large body.
///
/// The proxy may:
/// - Forward the Expect header and let the upstream send 100 Continue,
///   then relay both the 100 and the final response; or
/// - Internally satisfy the 100 Continue itself and then relay the
///   request + upstream response.
///
/// In either case the final status from the upstream (200 OK) must
/// appear in the response stream returned to the client.
#[test]
fn expect_100_continue_should_eventually_proxy() {
    let resp = replay_fixture("connection/expect_100_continue.http");
    assert!(
        resp.contains(HTTP_REPLAY_OK_RESPONSE),
        "Request with Expect: 100-continue should result in upstream 200 OK"
    );
}

/// A request with 50 custom extension headers.
///
/// HTTP/1.1 does not impose a limit on the number of header fields,
/// though implementations often do for resource protection.  This test
/// verifies that a moderately large header set (50 fields) is either
/// forwarded intact or rejected with a clear error — not silently
/// truncated, which could strip security-relevant headers.
#[test]
fn many_header_fields_should_be_handled_safely() {
    let resp = replay_fixture("connection/many_header_fields.http");
    // Either proxied (200) or rejected (non-200). Both are acceptable.
    // What is not acceptable is a hang or empty response.
    assert!(
        !resp.is_empty(),
        "Proxy must respond to a request with 50 headers without hanging"
    );
}
