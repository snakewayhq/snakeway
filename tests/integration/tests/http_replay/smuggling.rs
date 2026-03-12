use super::replay_fixture;
use integration::constants::HTTP_REPLAY_OK_RESPONSE;

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
