use http::Version;
use pingora::prelude::Session;
use pingora::protocols::http::ServerSession;

/// Detects CL.TE / TE.CL smuggling attempts that Pingora's HTTP/1 parser has partially handled.
///
/// When a request carries both `Content-Length` and `Transfer-Encoding`, Pingora strips CL
/// (RFC 9112 §6.3) and disables keepalive on the session (RFC 9112 §6.1-15). Since CL is gone
/// by the time we run, we infer CL+TE from the keepalive flag instead.
///
/// For an HTTP/1.1 request that didn't send `Connection: close` and still has reuse budget,
/// Pingora leaves keepalive on by default. Keepalive being off under those conditions means
/// the CL+TE detection path fired. We read that via `ServerSession::H1.will_keepalive()`.
///
/// We filter out the other keepalive-off cases that would false-positive:
///   * HTTP/1.0 defaults to keepalive-off, and Pingora already rejects HTTP/1.0 carrying
///     `Transfer-Encoding`.
///   * An exhausted reuse counter also leaves `will_keepalive()` false once
///     reuses_remaining reaches 0.
///
/// Caveat: this relies on Pingora's current internals, not a stable API. Revisit on upgrade.
pub(in crate::proxy) fn is_cl_te_smuggling_attempt(session: &Session) -> bool {
    let req = session.req_header();

    // Only HTTP/1.1 is checked. HTTP/1.0 defaults to keepalive-off and would
    // false-positive, and Pingora's validate_request already rejects HTTP/1.0 carrying
    // Transfer-Encoding.
    if req.version != Version::HTTP_11 {
        return false;
    }

    if !req.headers.contains_key("transfer-encoding") {
        return false;
    }

    let client_closed = req
        .headers
        .get("connection")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .split(',')
        .any(|t| t.trim().eq_ignore_ascii_case("close"));

    if client_closed {
        return false;
    }

    match session.downstream_session.as_ref() {
        ServerSession::H1(h1) => {
            // Exclude the reuse-counter-exhausted case, which also turns keepalive off.
            if h1.get_keepalive_reuses_remaining() == Some(0) {
                return false;
            }
            !h1.will_keepalive()
        }
        _ => false,
    }
}
