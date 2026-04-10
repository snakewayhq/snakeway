use super::replay_fixture;
use snakeway_tests::constants::HTTP_REPLAY_OK_RESPONSE;

/// CL.TE request smuggling test.
///
/// Technique:
/// A request includes BOTH `Content-Length` and `Transfer-Encoding: chunked`.
///
/// According to RFC 9112:
///     Transfer-Encoding takes precedence over Content-Length.
///
/// Historically many proxies used Content-Length while the backend server
/// used Transfer-Encoding. That disagreement allows an attacker to hide a
/// second HTTP request inside the body.
///
/// Example attack flow:
///
/// client → proxy → backend
///
/// Proxy interpretation:
///     uses Content-Length
///     reads 13 bytes of body
///
/// Backend interpretation:
///     uses Transfer-Encoding
///     sees chunk "0" (end of body)
///     treats the remaining bytes as a SECOND request
///
/// That second request executes with the same connection context.
///
/// This test verifies that Snakeway does NOT proxy such a request upstream.
#[test]
fn cl_te_smuggling_should_be_rejected() {
    let resp = replay_fixture("smuggling/cl_te.http");

    assert!(
        !resp.contains(HTTP_REPLAY_OK_RESPONSE),
        "CL.TE smuggling attempt should not be accepted or proxied"
    );
}

/// TE.CL request smuggling test.
///
/// Technique:
/// Same idea as CL.TE but the header order is reversed:
///
///     Transfer-Encoding: chunked
///     Content-Length: 4
///
/// Some proxies historically trusted the FIRST header they saw while
/// backend servers trusted Transfer-Encoding regardless of order.
///
/// If the proxy trusts Content-Length while the backend trusts
/// Transfer-Encoding, the request boundaries differ and a hidden
/// request can be injected.
///
/// This test ensures Snakeway does not forward such ambiguous requests.
#[test]
fn te_cl_smuggling_should_be_rejected() {
    let resp = replay_fixture("smuggling/te_cl.http");

    assert!(
        !resp.contains(HTTP_REPLAY_OK_RESPONSE),
        "TE.CL smuggling attempt should not be accepted or proxied"
    );
}

/// Dual Content-Length request smuggling test.
///
/// Technique:
/// The request includes two Content-Length headers:
///
///     Content-Length: 5
///     Content-Length: 10
///
/// Different parsers may choose:
///     - first value
///     - last value
///     - reject the request
///
/// If the proxy and backend choose different values,
/// the backend may read extra bytes as the start of
/// a second hidden request.
///
/// Modern secure proxies MUST reject requests with
/// conflicting Content-Length headers.
///
/// This test verifies Snakeway does not accept such
/// malformed requests.
#[test]
fn dual_content_length_should_be_rejected() {
    let resp = replay_fixture("smuggling/dual_content_length.http");

    assert!(
        !resp.contains(HTTP_REPLAY_OK_RESPONSE),
        "Duplicate Content-Length must be rejected"
    );
}

/// Obfuscated Transfer-Encoding smuggling test.
///
/// Technique:
/// Use a non-standard Transfer-Encoding token (`xchunked`) that some
/// upstream servers normalise to `chunked` while the proxy rejects or
/// ignores the header and falls back to Content-Length.
///
/// If the proxy uses Content-Length (5 bytes = "hello") while the
/// backend treats the unknown TE as chunked and reads `hello\r\n0\r\n`
/// as two message parts, the request boundary shifts and a hidden
/// second request can be injected.
///
/// Snakeway must not proxy a request with an unrecognised
/// Transfer-Encoding alongside a Content-Length.
#[test]
fn te_obfuscated_chunked_should_be_rejected() {
    let resp = replay_fixture("smuggling/te_chunked_obfuscated.http");

    assert!(
        !resp.contains(HTTP_REPLAY_OK_RESPONSE),
        "Obfuscated Transfer-Encoding with Content-Length must not be proxied"
    );
}

/// Leading-whitespace Transfer-Encoding smuggling variant.
///
/// Technique:
/// `Transfer-Encoding:  chunked` (two spaces before the value) is a
/// known bypass against WAFs and proxies that whitespace-strip header
/// values inconsistently.  If the proxy doesn't recognise the
/// double-spaced value as `chunked` but the backend does, the proxy
/// may fall back to Content-Length framing, creating a desync.
///
/// A correctly implemented proxy must either normalise header-value
/// whitespace and detect the conflict, or reject the ambiguous request.
#[test]
fn te_with_leading_whitespace_cl_should_be_rejected() {
    let resp = replay_fixture("smuggling/te_space_before_value.http");

    assert!(
        !resp.contains(HTTP_REPLAY_OK_RESPONSE),
        "Transfer-Encoding with leading whitespace + Content-Length desync must be rejected"
    );
}

/// Chunked-then-Content-Length pipeline injection.
///
/// Technique:
/// A request that carries Transfer-Encoding: chunked with a chunked
/// body that terminates immediately (chunk size 0) but also declares
/// Content-Length: 20.
///
/// The 20 bytes declared by Content-Length happen to be the start of
/// a second HTTP request (`POST /api ...`). A proxy that gives precedence
/// to Content-Length after seeing a zero-length chunked body would forward
/// those 20 bytes as the body, letting the hidden request prefix "bleed"
/// into the next upstream connection slot.
///
/// RFC 9112 requires that when both Transfer-Encoding and Content-Length
/// are present, Transfer-Encoding takes precedence. The proxy must
/// reject or safely handle this ambiguity rather than forwarding the
/// Content-Length count worth of bytes as a body.
#[test]
fn chunked_body_with_content_length_should_be_rejected() {
    let resp = replay_fixture("smuggling/chunked_then_cl.http");

    assert!(
        !resp.contains(HTTP_REPLAY_OK_RESPONSE),
        "Chunked body combined with conflicting Content-Length must be rejected"
    );
}
