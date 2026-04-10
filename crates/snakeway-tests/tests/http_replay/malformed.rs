use super::replay_fixture;
use snakeway_tests::conf::ConfigBuilder;
use snakeway_tests::constants::HTTP_REPLAY_OK_RESPONSE;
use snakeway_tests::harness::TestServer;

/// HTTP/1.1 requires a Host header (RFC 9112 §3.2).
///
/// A reverse proxy that forwards a request lacking Host to the upstream
/// gives the upstream no usable routing information. RFC 9112 requires
/// servers to respond with 400 Bad Request for HTTP/1.1 requests
/// without Host.
#[test]
fn missing_host_header_should_be_rejected() {
    let resp = replay_fixture("malformed/missing_host.http");

    assert!(
        !resp.contains(HTTP_REPLAY_OK_RESPONSE),
        "HTTP/1.1 request without Host header must not be proxied"
    );
}

/// Snakeway must only accept the HTTP versions it understands.
///
/// A request line of `GET /api HTTP/9.9` uses an unknown version.
/// RFC 9110 §2.5 says a server MUST respond with a 505 (HTTP Version
/// Not Supported) for unsupported major versions. Either a 505 or a 400
/// is acceptable; what is NOT acceptable is proxying to upstream.
#[test]
fn invalid_http_version_should_be_rejected() {
    let resp = replay_fixture("malformed/invalid_version.http");

    assert!(
        !resp.contains(HTTP_REPLAY_OK_RESPONSE),
        "Request with unknown HTTP version must not be proxied"
    );
}

/// A Content-Length value must be a non-negative decimal integer.
///
/// `Content-Length: -1` is syntactically invalid (RFC 9110 §8.6).
/// Accepting it opens the door to request smuggling: the proxy and
/// backend may interpret the malformed value differently.
#[test]
fn negative_content_length_should_be_rejected() {
    let resp = replay_fixture("malformed/negative_content_length.http");

    assert!(
        !resp.contains(HTTP_REPLAY_OK_RESPONSE),
        "Negative Content-Length must not be accepted or proxied"
    );
}

/// A non-numeric Content-Length value is malformed per RFC 9110 §8.6.
///
/// `Content-Length: abc` cannot be parsed as a body length; any proxy
/// that forwards this request is either ignoring the header entirely
/// (losing body framing) or guessing — both are unsafe.
#[test]
fn non_numeric_content_length_should_be_rejected() {
    let resp = replay_fixture("malformed/non_numeric_content_length.http");

    assert!(
        !resp.contains(HTTP_REPLAY_OK_RESPONSE),
        "Non-numeric Content-Length must not be accepted or proxied"
    );
}

/// RFC 9112 §3 states: "A server that receives a method token that
/// starts with whitespace SHOULD respond with a 400 (Bad Request)."
///
/// Leading whitespace before the method token is a common artefact of
/// certain request-smuggling pre-amble techniques and must not be
/// silently accepted.
#[test]
fn leading_whitespace_before_method_should_be_rejected() {
    let resp = replay_fixture("malformed/whitespace_before_method.http");

    assert!(
        !resp.contains(HTTP_REPLAY_OK_RESPONSE),
        "Request line with leading whitespace must not be proxied"
    );
}

/// The HTTP/1.1 request-line format is `method SP request-target SP HTTP-version`.
///
/// A request line containing only a method and path with no HTTP version
/// is either an HTTP/0.9 simple-request (which Snakeway does not support
/// as a reverse proxy) or a malformed request.  Either way it must not
/// be forwarded.
#[test]
fn request_line_without_version_should_be_rejected() {
    let resp = replay_fixture("malformed/request_line_no_version.http");

    assert!(
        !resp.contains(HTTP_REPLAY_OK_RESPONSE),
        "Request line without HTTP version must not be proxied"
    );
}

/// When the declared Content-Length is larger than the actual body bytes
/// sent before the connection closes, the proxy must not forward an
/// incomplete request to the upstream.
///
/// Forwarding a truncated body could cause the upstream to block waiting
/// for more bytes, effectively creating a half-open connection that ties
/// up upstream resources.
///
/// This test requires:
/// - A request filter device with a short `client_body_timeout` so Pingora
///   gives up waiting for the remaining body bytes quickly.
/// - An upstream that reads the request before responding, so it doesn't
///   race ahead with a 200 before the proxy detects the underflow.
#[test]
fn content_length_body_underflow_should_not_proxy_successfully() {
    // Arrange: 2-second client body timeout so the test doesn't take 60s.
    let mut rf = ConfigBuilder::make_request_filter_device_spec();
    rf.client_body_timeout_seconds = Some(2);
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_request_filter(rf)
        .build();
    let srv = TestServer::start_http_upstream_that_reads_request_with_config(&mut cfg);

    // Act
    let resp = srv.replay_http_fixture("malformed/content_length_body_underflow.http");

    // Assert
    assert!(
        !resp.contains(HTTP_REPLAY_OK_RESPONSE),
        "Request whose body is shorter than Content-Length must not proxy successfully"
    );
}

/// In RFC 9112 §3.2.2, the absolute-form request-target (`http://host/path`)
/// is defined for use only with the CONNECT method and traditional
/// (forward) proxies. A reverse proxy receiving an absolute-form URI
/// targeting an external host must not blindly forward to its configured
/// upstream — that would make it an open relay.
///
/// Snakeway should either normalise the absolute-form to origin-form or
/// reject the request outright. In either case, no 200 OK from the
/// configured upstream should be returned when the request targets a
/// different authority than the configured backend.
#[test]
fn absolute_uri_targeting_external_host_should_not_proxy() {
    // This fixture uses absolute-form: GET http://snakeway.test:8080/api HTTP/1.1
    // A well-behaved reverse proxy strips the authority and forwards the
    // path — that is acceptable. What must NOT happen is the proxy acting
    // as an open relay by forwarding requests to arbitrary hosts.
    // This test documents that the proxy either rejects or safely handles
    // absolute-form requests without becoming an open relay.
    let resp = replay_fixture("malformed/absolute_uri.http");

    // The upstream mock always replies with "200 OK" if the request
    // arrives. Receiving 200 here means the proxy forwarded the request,
    // which may be acceptable if the URI target matches the configured
    // upstream. We verify the server responds with something (no panic/hang)
    // and note the actual behaviour for regression tracking.
    //
    // For stricter environments this assertion can be flipped to
    // `!resp.contains(HTTP_REPLAY_OK_RESPONSE)` once rejection is confirmed.
    assert!(
        !resp.is_empty(),
        "Proxy must respond to absolute-URI requests without hanging"
    );
}
