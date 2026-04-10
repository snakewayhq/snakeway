use super::replay_fixture;
use snakeway_tests::constants::HTTP_REPLAY_OK_RESPONSE;

/// A minimal, perfectly-formed GET request — the most fundamental
/// proxy operation.  If this fails everything else is moot.
#[test]
fn get_minimal_should_proxy() {
    let resp = replay_fixture("methods/get_minimal.http");
    assert!(
        resp.contains(HTTP_REPLAY_OK_RESPONSE),
        "Minimal GET should be proxied to upstream"
    );
}

/// POST with a JSON body and a correct Content-Length.
///
/// Verifies that the proxy correctly frames and forwards request bodies
/// rather than discarding them or corrupting the body length.
#[test]
fn post_with_json_body_should_proxy() {
    let resp = replay_fixture("methods/post_json.http");
    assert!(
        resp.contains(HTTP_REPLAY_OK_RESPONSE),
        "POST with JSON body and correct Content-Length should be proxied"
    );
}

/// PUT is semantically identical to POST from the proxy's perspective.
///
/// Some middleware layers or WAF rules treat PUT differently from POST;
/// this confirms the proxy does not drop or block PUT requests by default.
#[test]
fn put_with_body_should_proxy() {
    let resp = replay_fixture("methods/put_update.http");
    assert!(
        resp.contains(HTTP_REPLAY_OK_RESPONSE),
        "PUT with a body should be proxied"
    );
}

/// DELETE with no body is a straightforward mutation method.
///
/// Proxies should not require a body for DELETE to be forwarded; some
/// implementations mistakenly require Content-Length for any method
/// that is not GET/HEAD.
#[test]
fn delete_without_body_should_proxy() {
    let resp = replay_fixture("methods/delete_resource.http");
    assert!(
        resp.contains(HTTP_REPLAY_OK_RESPONSE),
        "DELETE without a body should be proxied"
    );
}

/// HEAD is defined by RFC 9110 §9.3.2 to be identical to GET except
/// the server must not send a response body.
///
/// A reverse proxy must forward HEAD and return the upstream headers.
/// The upstream mock will respond with its normal 200 status line;
/// the proxy must not suppress or transform the status.
#[test]
fn head_request_should_proxy() {
    let resp = replay_fixture("methods/head_request.http");
    assert!(
        resp.contains(HTTP_REPLAY_OK_RESPONSE),
        "HEAD request should be proxied and return 200 status"
    );
}

/// OPTIONS is used for CORS preflight in browsers and for capability
/// discovery in REST APIs.
///
/// A reverse proxy must forward OPTIONS requests with their associated
/// CORS request headers (Origin, Access-Control-Request-Method, etc.)
/// intact so that the upstream can generate appropriate CORS response
/// headers.
#[test]
fn options_cors_preflight_should_proxy() {
    let resp = replay_fixture("methods/options_cors_preflight.http");
    assert!(
        resp.contains(HTTP_REPLAY_OK_RESPONSE),
        "OPTIONS CORS preflight should be proxied"
    );
}

/// PATCH (RFC 5789) applies a partial modification to a resource.
///
/// It is less common than GET/POST/PUT but still a standard method
/// that a reverse proxy must forward without mangling the body.
#[test]
fn patch_with_body_should_proxy() {
    let resp = replay_fixture("methods/patch_update.http");
    assert!(
        resp.contains(HTTP_REPLAY_OK_RESPONSE),
        "PATCH with a body should be proxied"
    );
}

/// RFC 9110 §9.3.1 does not prohibit a body on a GET request, though
/// it notes that the semantics are undefined.  Some API clients
/// (notably Elasticsearch) send GET with a JSON body.
///
/// A proxy must not discard or reject the body solely because the
/// method is GET; it should forward whatever the client sends.
#[test]
fn get_with_body_should_proxy() {
    let resp = replay_fixture("methods/get_with_body.http");
    assert!(
        resp.contains(HTTP_REPLAY_OK_RESPONSE),
        "GET with a request body should still be proxied"
    );
}
