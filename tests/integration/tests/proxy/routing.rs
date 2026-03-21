use integration::conf::minimal_http_runtime_config;
use integration::harness::TestServer;
use pretty_assertions::assert_eq;
use reqwest::StatusCode;

//-----------------------------------------------------------------------------
// Longest-prefix-match semantics
//-----------------------------------------------------------------------------

/// A route configured for `/api` must also match sub-paths like
/// `/api/users/123`.
///
/// Snakeway uses longest-prefix-match routing per RFC 9110 conventions.
/// This test confirms that sub-paths are not treated as distinct routes
/// and are correctly routed to the upstream that owns the prefix.
#[test]
fn path_prefix_matches_sub_path() {
    // Arrange
    let mut cfg = minimal_http_runtime_config();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Act
    let res = srv.get("/api/users/123").send().unwrap();

    // Assert
    assert_eq!(res.status(), StatusCode::OK);
}

/// A request to `/apiv2` must NOT match a route configured only for `/api`.
///
/// Prefix matching must not conflate `/api` with `/apiv2`. The prefix
/// `/api` requires either an exact match or a path separator (`/`) after
/// the prefix — not an arbitrary alphanumeric continuation.
#[test]
fn path_prefix_does_not_match_sibling_prefix() {
    // Arrange
    let mut cfg = minimal_http_runtime_config();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Act
    let res = srv.get("/apiv2").send().unwrap();

    // Assert
    assert_eq!(
        res.status(),
        StatusCode::NOT_FOUND,
        "/apiv2 must not be matched by /api route"
    );
}

/// A request path that is shorter than the configured route prefix must
/// not match.
///
/// `/ap` is a proper prefix of `/api`, not the other way around.
/// The router must not match a request whose path is shorter than the
/// configured route's path.
#[test]
fn path_prefix_does_not_match_parent() {
    // Arrange
    let mut cfg = minimal_http_runtime_config();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Act
    let res = srv.get("/ap").send().unwrap();

    // Assert
    assert_eq!(
        res.status(),
        StatusCode::NOT_FOUND,
        "/ap must not be matched by /api route"
    );
}

/// A request to the root path `/` must return 404 when no root route
/// is configured.
///
/// Some proxy implementations fall back to a default upstream when no
/// route matches. Snakeway should return 404 explicitly, ensuring
/// unrouted traffic is never silently forwarded.
#[test]
fn root_path_without_route_returns_404() {
    // Arrange
    let mut cfg = minimal_http_runtime_config();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Act
    let res = srv.get("/").send().unwrap();

    // Assert
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

/// A request path with a trailing slash (`/api/`) must still be matched
/// by the `/api` route prefix.
///
/// Trailing slashes on sub-resource collections (`/api/`) are common in
/// REST APIs. The proxy must not reject them solely because the
/// configured prefix lacks a trailing slash.
#[test]
fn trailing_slash_on_path_prefix_is_matched() {
    // Arrange
    let mut cfg = minimal_http_runtime_config();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Act
    let res = srv.get("/api/").send().unwrap();

    // Assert
    assert_eq!(
        res.status(),
        StatusCode::OK,
        "/api/ should be matched by the /api route"
    );
}
