use super::replay_fixture;
use snakeway_tests::conf::minimal_http_runtime_config;
use snakeway_tests::constants::HTTP_REPLAY_OK_RESPONSE;
use snakeway_tests::harness::TestServer;

/// Percent-encoded characters in the path must be forwarded intact.
///
/// RFC 3986 §2.1 defines percent-encoding as a way to include characters
/// that would otherwise be interpreted as delimiters.  A proxy must not
/// decode-then-re-encode the path, which would change the semantics of
/// encoded slashes (%2F) and spaces (%20).
#[test]
fn encoded_path_should_proxy() {
    let resp = replay_fixture("uri/encoded_path.http");
    assert!(
        resp.contains(HTTP_REPLAY_OK_RESPONSE),
        "Path with percent-encoded characters should be proxied"
    );
}

/// Query strings are a fundamental part of REST API URLs.
///
/// A proxy must forward the full query string without truncation,
/// double-encoding, or stripping of parameters.  This fixture exercises
/// `+` encoding for spaces and multiple parameters.  The upstream echoes
/// the request line it received, so the assertion covers what the proxy
/// actually forwarded rather than only the response status.
#[test]
fn query_string_should_proxy() {
    // Arrange
    let mut cfg = minimal_http_runtime_config();
    let srv = TestServer::start_http_upstream_that_echoes_request_line_with_config(&mut cfg);

    // Act
    let resp = srv.replay_http_fixture("uri/query_string.http");

    // Assert
    assert!(
        resp.contains(HTTP_REPLAY_OK_RESPONSE),
        "Request with query string should be proxied; got: {resp}"
    );
    assert!(
        resp.contains("GET /api?action=search&q=hello+world&page=1&limit=50"),
        "Upstream must receive the full query string in the request line; upstream saw: {resp}"
    );
}

/// An empty query string (`/api?`) is syntactically valid per RFC 3986.
///
/// Some proxy implementations discard the trailing `?` or fail to
/// parse the path correctly when there are no query parameters.
#[test]
fn empty_query_string_should_proxy() {
    let resp = replay_fixture("uri/empty_query_string.http");
    assert!(
        resp.contains(HTTP_REPLAY_OK_RESPONSE),
        "Request with empty query string (trailing ?) should be proxied"
    );
}

/// A query string approaching 2 KB tests the proxy's URL length handling.
///
/// Proxies often impose limits on URL length; this verifies that a
/// realistic but long query string is forwarded rather than silently
/// truncated or rejected without a meaningful error.
#[test]
fn long_query_string_should_proxy() {
    let resp = replay_fixture("uri/long_query_string.http");
    assert!(
        resp.contains(HTTP_REPLAY_OK_RESPONSE),
        "Request with a long query string should be proxied"
    );
}

/// Paths containing dot-segments (`..`) should be normalised or
/// forwarded safely.
///
/// RFC 3986 §5.2.4 specifies that dot-segments must be removed during
/// resolution. A proxy that forwards `/api/../api` verbatim is
/// acceptable if the upstream handles normalisation; what must NOT
/// happen is the proxy crashing or returning a 500.
#[test]
fn dot_segment_path_should_be_handled_safely() {
    let resp = replay_fixture("uri/dot_segment_path.http");
    // Either normalised to /api and proxied (200) or rejected with an
    // error (non-200) — either is a valid proxy behaviour.
    // What is not acceptable is a hang or an empty response.
    assert!(
        !resp.is_empty(),
        "Proxy must respond to dot-segment paths without hanging"
    );
}

/// Percent-encoded Unicode characters in query strings must be forwarded
/// verbatim.  APIs that accept internationalised inputs rely on the
/// proxy preserving UTF-8 encoded sequences such as CJK characters
/// (%E4%B8%AD), emoji (%F0%9F%91%8D), and Latin accented chars (%C3%A9).
#[test]
fn unicode_encoded_query_should_proxy() {
    let resp = replay_fixture("uri/unicode_encoded_query.http");
    assert!(
        resp.contains(HTTP_REPLAY_OK_RESPONSE),
        "Request with Unicode percent-encoded query parameters should be proxied"
    );
}

/// A null byte encoded as `%00` in the path is a classic injection
/// technique exploited against C-string–based path handling and some
/// filesystem access routines.
///
/// A reverse proxy must either reject the request or forward the
/// percent-encoded null byte intact (NOT decode it to a literal `\0`
/// before forwarding). Either policy is acceptable; what is
/// unacceptable is decoding the null byte and forwarding a raw `\0`
/// to the upstream, which could truncate the path string in some
/// backends.
///
/// This test documents that the proxy at minimum does not hang or crash.
#[test]
fn null_byte_encoded_in_path_should_be_handled_safely() {
    let resp = replay_fixture("uri/null_byte_in_path.http");
    assert!(
        !resp.is_empty(),
        "Proxy must respond to %00 in path without crashing"
    );
}

/// Double-encoded path traversal (`%2e%2e` for `..`) is a well-known
/// bypass for path-traversal filters that only check for literal `../`.
///
/// A proxy must not decode percent-encoded dot-segments and then re-
/// evaluate the path, which would allow the bypass. It should either:
/// - Forward the path as-is (letting the upstream handle it), or
/// - Reject the request with a 4xx error.
///
/// In both cases the proxy must not crash.
#[test]
fn double_encoded_path_traversal_should_be_handled_safely() {
    let resp = replay_fixture("uri/path_traversal_encoded.http");
    assert!(
        !resp.is_empty(),
        "Proxy must respond to double-encoded path traversal without crashing"
    );
}
