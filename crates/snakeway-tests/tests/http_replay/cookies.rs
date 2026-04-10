use super::replay_fixture;
use snakeway_tests::constants::HTTP_REPLAY_OK_RESPONSE;

/// A single Cookie header with a multi-kilobyte value.
///
/// Some proxies impose per-header-line limits that can truncate large
/// session cookies, silently breaking authentication for users with
/// many active sessions or large JWT payloads stored in cookies.
#[test]
fn large_cookie_should_proxy() {
    let resp = replay_fixture("cookies/large_cookie.http");
    assert!(resp.contains(HTTP_REPLAY_OK_RESPONSE));
}

/// A Cookie header carrying 50 distinct name=value pairs.
///
/// RFC 6265 §5.4 places no limit on the number of cookies a client may
/// send in a single Cookie header. Some proxies split or truncate the
/// cookie list at an arbitrary count. This test verifies that a
/// moderately large cookie jar is forwarded intact.
#[test]
fn many_cookies_should_proxy() {
    let resp = replay_fixture("cookies/many_cookies.http");
    assert!(
        resp.contains(HTTP_REPLAY_OK_RESPONSE),
        "Request with 50 cookies should be proxied"
    );
}

/// A Cookie header with values that contain special characters:
/// quoted-string values, percent-encoded characters, an empty value,
/// and a value with an equals sign inside.
///
/// RFC 6265 §4.1.1 allows cookie values to be optionally enclosed in
/// double quotes and to contain most US-ASCII characters except control
/// characters, whitespace, double quotes, comma, semicolon, and
/// backslash. A proxy must not attempt to re-parse or normalise cookie
/// values — it should pass them through opaquely.
#[test]
fn cookie_special_chars_should_proxy() {
    let resp = replay_fixture("cookies/cookie_special_chars.http");
    assert!(
        resp.contains(HTTP_REPLAY_OK_RESPONSE),
        "Request with special-character cookie values should be proxied"
    );
}
