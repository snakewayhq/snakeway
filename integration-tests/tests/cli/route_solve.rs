use integration_tests::conf::ConfigBuilder;
use integration_tests::constants::{ROUTE_PATH_API, TEST_HOST};
use pretty_assertions::assert_eq;
use snakeway_core::integration_test_api::cli::route::solve::{
    RouteSolveOptions, SyntheticRequest, walk_solve,
};
use snakeway_core::integration_test_api::control_plane::runtime::build_runtime_state;

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

fn opts(trace: bool) -> RouteSolveOptions {
    RouteSolveOptions {
        lb_key: None,
        lb_index: None,
        trace,
        verbose: false,
    }
}

#[test]
fn route_solve_matches_service_route() {
    // Arrange
    let cfg = ConfigBuilder::default().with_http_ingress().build();
    let state = build_runtime_state(&cfg, &None).expect("build_runtime_state failed");
    let req = make_req("http", TEST_HOST, "/api/users");

    // Act
    let decision = walk_solve(&state, &req, &opts(false));

    // Assert
    assert!(decision.matched_route.is_some(), "should match a route");
    assert_eq!(decision.route_kind.as_deref(), Some("service"));
    assert!(
        decision.selected_upstream.is_some(),
        "should select an upstream"
    );
    assert!(decision.rejection.is_none(), "should not be rejected");
}

#[test]
fn route_solve_no_match_returns_rejection() {
    // Arrange
    let cfg = ConfigBuilder::default().with_http_ingress().build();
    let state = build_runtime_state(&cfg, &None).expect("build_runtime_state failed");
    let req = make_req("http", TEST_HOST, "/nonexistent");

    // Act
    let decision = walk_solve(&state, &req, &opts(false));

    // Assert
    assert!(decision.matched_route.is_none());
    assert!(decision.rejection.is_some());
    assert_eq!(decision.rejection.as_ref().unwrap().stage, "route_match");
}

#[test]
fn route_solve_json_output_is_stable() {
    // Arrange
    let cfg = ConfigBuilder::default().with_http_ingress().build();
    let state = build_runtime_state(&cfg, &None).expect("build_runtime_state failed");
    let req = make_req("http", TEST_HOST, "/api/test");

    // Act
    let d1 = walk_solve(&state, &req, &opts(true));
    let d2 = walk_solve(&state, &req, &opts(true));

    // Assert
    let json1 = serde_json::to_string_pretty(&d1).unwrap();
    let json2 = serde_json::to_string_pretty(&d2).unwrap();
    assert_eq!(json1, json2, "JSON output must be deterministic");
}

#[test]
fn route_solve_lb_index_selects_correct_upstream() {
    // Arrange
    let cfg = ConfigBuilder::default().with_http_ingress().build();
    let state = build_runtime_state(&cfg, &None).expect("build_runtime_state failed");
    let req = make_req("http", TEST_HOST, ROUTE_PATH_API);
    let opts_0 = RouteSolveOptions {
        lb_key: None,
        lb_index: Some(0),
        trace: false,
        verbose: false,
    };
    let opts_1 = RouteSolveOptions {
        lb_key: None,
        lb_index: Some(1),
        trace: false,
        verbose: false,
    };

    // Act
    let d0 = walk_solve(&state, &req, &opts_0);
    let d1 = walk_solve(&state, &req, &opts_1);

    // Assert
    assert!(d0.selected_upstream.is_some());
    assert!(d1.selected_upstream.is_some());
    assert_ne!(
        d0.selected_upstream, d1.selected_upstream,
        "different lb_index should select different upstreams"
    );
}

#[test]
fn route_solve_lb_key_deterministic_across_calls() {
    // Arrange
    let cfg = ConfigBuilder::default().with_http_ingress().build();
    let state = build_runtime_state(&cfg, &None).expect("build_runtime_state failed");
    let req = make_req("http", TEST_HOST, ROUTE_PATH_API);
    let opts = RouteSolveOptions {
        lb_key: Some("session-abc-123".into()),
        lb_index: None,
        trace: false,
        verbose: false,
    };

    // Act
    let d1 = walk_solve(&state, &req, &opts);
    let d2 = walk_solve(&state, &req, &opts);

    // Assert
    assert_eq!(
        d1.selected_upstream, d2.selected_upstream,
        "same lb_key must yield same upstream"
    );
}

#[test]
fn route_solve_trace_contains_expected_stages() {
    // Arrange
    let cfg = ConfigBuilder::default().with_http_ingress().build();
    let state = build_runtime_state(&cfg, &None).expect("build_runtime_state failed");
    let req = make_req("http", TEST_HOST, ROUTE_PATH_API);

    // Act
    let decision = walk_solve(&state, &req, &opts(true));

    // Assert
    let trace = decision.trace.expect("trace should be present");
    assert!(!trace.is_empty(), "trace should have entries");

    let stages: Vec<&str> = trace.iter().map(|s| s.stage.as_str()).collect();
    assert!(
        stages.contains(&"route_match"),
        "trace should contain route_match stage"
    );
    assert!(
        stages.contains(&"upstream_select"),
        "trace should contain upstream_select stage"
    );
}
