use pretty_assertions::assert_eq;
use snakeway::testing_api::cli::route::solve::{RouteSolveOptions, SyntheticRequest, walk_solve};
use snakeway::testing_api::engine::runtime::build_runtime_state;
use snakeway_tests::conf::ConfigBuilder;
use snakeway_tests::constants::{ROUTE_PATH_GRPC, ROUTE_PATH_WS, TEST_HOST};

fn make_req(scheme: &str, host: &str, path: &str) -> SyntheticRequest {
    SyntheticRequest {
        scheme: scheme.into(),
        host: host.into(),
        method: http::Method::GET,
        path: path.to_string(),
        query: None,
        client_ip: None,
        body_size: 0,
    }
}

fn no_trace() -> RouteSolveOptions {
    RouteSolveOptions {
        lb_key: None,
        lb_index: None,
        trace: false,
        verbose: false,
    }
}

//-----------------------------------------------------------------------------
// WebSocket route
//-----------------------------------------------------------------------------

/// A request to the WebSocket route path must be matched by the WS
/// service route and have an upstream selected.
///
/// The CLI solver must correctly traverse WebSocket-enabled route configs
/// — not just plain HTTP service routes.
#[test]
fn route_solve_matches_ws_route() {
    // Arrange
    let cfg = ConfigBuilder::default().with_ws_ingress().build();
    let state = build_runtime_state(&cfg, &None).expect("build_runtime_state failed");
    let req = make_req("http", TEST_HOST, ROUTE_PATH_WS);

    // Act
    let decision = walk_solve(&state, &req, &no_trace());

    // Assert
    assert!(
        decision.matched_route.is_some(),
        "WebSocket route must match the configured WS path"
    );
    assert_eq!(decision.route_kind.as_deref(), Some("service"));
    assert!(
        decision.selected_upstream.is_some(),
        "an upstream must be selected for the WS route"
    );
    assert!(decision.rejection.is_none());
}

//-----------------------------------------------------------------------------
// Static file route
//-----------------------------------------------------------------------------

/// A request to the static file root must be matched by the static route.
///
/// Static file routes have no upstream — the route solver must not panic
/// or return a rejection just because `selected_upstream` is None.
#[test]
fn route_solve_matches_static_file_route() {
    // Arrange
    let cfg = ConfigBuilder::default()
        .with_static_file_ingress(false)
        .build();
    let state = build_runtime_state(&cfg, &None).expect("build_runtime_state failed");
    let req = make_req("http", TEST_HOST, "/index.html");

    // Act
    let decision = walk_solve(&state, &req, &no_trace());

    // Assert
    assert!(
        decision.matched_route.is_some(),
        "static file route must be matched"
    );
    assert!(
        decision.rejection.is_none(),
        "static file route must not be rejected"
    );
}

//-----------------------------------------------------------------------------
// Unmatched host
//-----------------------------------------------------------------------------

/// A request whose Host header does not match any configured virtual host
/// must produce a rejection at the `route_match` stage.
///
/// Host-based routing is a primary isolation mechanism in multi-tenant
/// reverse proxy deployments.  The solver must not fall through to a
/// default route when the host does not match.
#[test]
fn route_solve_unmatched_host_returns_rejection() {
    // Arrange
    let cfg = ConfigBuilder::default().with_http_ingress().build();
    let state = build_runtime_state(&cfg, &None).expect("build_runtime_state failed");
    let req = make_req("http", "unknown.example.com", "/api");

    // Act
    let decision = walk_solve(&state, &req, &no_trace());

    // Assert
    assert!(
        decision.matched_route.is_none(),
        "unknown host must not match any route"
    );
    assert!(
        decision.rejection.is_some(),
        "unknown host must produce a rejection"
    );
    assert_eq!(
        decision.rejection.as_ref().unwrap().stage,
        "route_match",
        "rejection must occur at route_match stage"
    );
}

//-----------------------------------------------------------------------------
// Unmatched path
//-----------------------------------------------------------------------------

/// A request whose path does not match any configured route must produce
/// a rejection at the `route_match` stage.
///
/// This is the path-level equivalent of the host mismatch test — a
/// different code path in the router.
#[test]
fn route_solve_unmatched_path_returns_rejection() {
    // Arrange
    let cfg = ConfigBuilder::default().with_http_ingress().build();
    let state = build_runtime_state(&cfg, &None).expect("build_runtime_state failed");
    let req = make_req("http", TEST_HOST, "/completely/unknown/path");

    // Act
    let decision = walk_solve(&state, &req, &no_trace());

    // Assert
    assert!(decision.matched_route.is_none());
    assert!(decision.rejection.is_some());
    assert_eq!(decision.rejection.as_ref().unwrap().stage, "route_match");
}

//-----------------------------------------------------------------------------
// gRPC route
//-----------------------------------------------------------------------------

/// A request to the gRPC service path must be matched by the gRPC
/// service route and have an upstream selected.
///
/// gRPC routes are HTTP/2-only; the solver must handle gRPC route
/// configs that use TLS upstreams without panicking.
#[test]
fn route_solve_grpc_route() {
    // Arrange
    let cfg = ConfigBuilder::default().with_grpc_ingress().build();
    let state = build_runtime_state(&cfg, &None).expect("build_runtime_state failed");
    let req = make_req("https", TEST_HOST, ROUTE_PATH_GRPC);

    // Act
    let decision = walk_solve(&state, &req, &no_trace());

    // Assert
    assert!(
        decision.matched_route.is_some(),
        "gRPC route path must be matched"
    );
    assert_eq!(decision.route_kind.as_deref(), Some("service"));
    assert!(
        decision.selected_upstream.is_some(),
        "an upstream must be selected for the gRPC route"
    );
    assert!(decision.rejection.is_none());
}
