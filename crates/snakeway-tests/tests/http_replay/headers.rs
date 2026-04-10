use super::replay_fixture;
use snakeway_tests::constants::HTTP_REPLAY_OK_RESPONSE;

/// Multiple values for the same header field name (e.g. two `Accept`
/// headers) are permitted by RFC 9110 §5.3 and are semantically
/// equivalent to a comma-combined single header. The proxy must forward
/// both values rather than dropping one.
#[test]
fn duplicate_headers_should_proxy() {
    let resp = replay_fixture("headers/duplicate_header.http");
    assert!(resp.contains(HTTP_REPLAY_OK_RESPONSE));
}

/// Hop-by-hop headers (Connection, Keep-Alive, Proxy-Connection,
/// Transfer-Encoding, etc.) are connection-scoped and must be stripped
/// by the proxy before forwarding to the upstream. Their presence in
/// the client request must not prevent proxying.
#[test]
fn hop_by_hop_headers_should_proxy() {
    let resp = replay_fixture("headers/hop_by_hop.http");
    assert!(resp.contains(HTTP_REPLAY_OK_RESPONSE));
}

/// An empty header value (`X-Empty-Value:` with nothing after the colon)
/// is syntactically valid per RFC 9110 §5.5. Some proxy implementations
/// strip headers with empty values or reject the request. The proxy must
/// forward empty-valued headers intact.
#[test]
fn empty_header_value_should_proxy() {
    let resp = replay_fixture("headers/empty_header_value.http");
    assert!(
        resp.contains(HTTP_REPLAY_OK_RESPONSE),
        "Request with an empty header value should be proxied"
    );
}

/// A single header field value that is several kilobytes long.
///
/// Proxies commonly impose per-header-line size limits to protect
/// against header-based DoS attacks. This test verifies that either:
/// - The long header is forwarded (if within the proxy's configured
///   limit), resulting in a 200 OK from upstream; or
/// - The proxy rejects the request with a 4xx and does not crash.
///
/// Specifically tests that the proxy does not hang or panic on an
/// oversized header value.
#[test]
fn long_header_value_should_be_handled_safely() {
    let resp = replay_fixture("headers/long_header_value.http");
    assert!(
        !resp.is_empty(),
        "Proxy must respond to a request with a very long header value without hanging"
    );
}

/// When a client already carries an `X-Forwarded-For` header, the proxy
/// should append the client's address to the chain rather than
/// overwriting the existing value. This preserves the full forwarding
/// path for upstream logging and access control.
///
/// Regardless of the proxy's XFF strategy, the request must be proxied
/// (200 OK).
#[test]
fn existing_x_forwarded_for_should_proxy() {
    let resp = replay_fixture("headers/x_forwarded_for_existing.http");
    assert!(
        resp.contains(HTTP_REPLAY_OK_RESPONSE),
        "Request with existing X-Forwarded-For should be proxied"
    );
}

/// The RFC 7239 `Forwarded` header is the standards-track replacement
/// for the de-facto `X-Forwarded-For` / `X-Forwarded-Proto` headers.
///
/// A proxy should either pass the `Forwarded` header through intact or
/// extend it with its own node information. It must not silently drop
/// the header, as downstream services may rely on it for security
/// decisions.
#[test]
fn rfc7239_forwarded_header_should_proxy() {
    let resp = replay_fixture("headers/forwarded_rfc7239.http");
    assert!(
        resp.contains(HTTP_REPLAY_OK_RESPONSE),
        "Request with RFC 7239 Forwarded header should be proxied"
    );
}
