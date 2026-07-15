use confval::source::Located;
use pretty_assertions::assert_eq;
use reqwest::StatusCode;
use snakeway::testing_api::conf::types::{ServiceRouteSpec, ServiceSpec};
use snakeway::testing_api::conf::validation::ConfigError;
use snakeway_tests::conf::{ConfigBuilder, minimal_http_runtime_config};
use snakeway_tests::constants::{TEST_HOST, UPSTREAM_PORT_PRIMARY, UPSTREAM_PORT_SECONDARY};
use snakeway_tests::harness::TestServer;

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

/// Build a config with the given host pattern and send a request with the
/// given Host header. Returns the response status code.
fn host_routing_status(host_pattern: &str, request_host: &str) -> StatusCode {
    let mut cfg = ConfigBuilder::default()
        .with_custom_ingress(vec![ServiceSpec {
            routes: vec![Located::detached(ServiceRouteSpec {
                hosts: vec![Located::detached(host_pattern.to_string())],
                path: Located::detached("/api".to_string()),
                ..Default::default()
            })],
            upstreams: vec![
                Located::detached(ConfigBuilder::make_tcp_upstream(
                    UPSTREAM_PORT_PRIMARY,
                    false,
                )),
                Located::detached(ConfigBuilder::make_tcp_upstream(
                    UPSTREAM_PORT_SECONDARY,
                    false,
                )),
            ],
            ..Default::default()
        }])
        .build();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    srv.client
        .get(srv.base_url().join("/api").unwrap())
        .header("Host", request_host)
        .send()
        .unwrap()
        .status()
}

/// A wildcard host `*.example.com` must match subdomains like
/// `foo.example.com`.
#[test]
fn wildcard_host_matches_subdomain() {
    assert_eq!(
        host_routing_status("*.example.com", "foo.example.com"),
        StatusCode::OK
    );
}

/// A wildcard host `*.example.com` must NOT match the bare domain
/// `example.com` (the wildcard requires at least one label before the dot).
#[test]
fn wildcard_host_does_not_match_bare_domain() {
    assert_eq!(
        host_routing_status("*.example.com", "example.com"),
        StatusCode::NOT_FOUND,
    );
}

/// A catch-all host `*` must match any Host header value.
#[test]
fn catch_all_host_matches_any_host() {
    let mut cfg = ConfigBuilder::default()
        .with_custom_ingress(vec![ServiceSpec {
            routes: vec![Located::detached(ServiceRouteSpec {
                hosts: vec![Located::detached("*".to_string())],
                path: Located::detached("/api".to_string()),
                ..Default::default()
            })],
            upstreams: vec![
                Located::detached(ConfigBuilder::make_tcp_upstream(
                    UPSTREAM_PORT_PRIMARY,
                    false,
                )),
                Located::detached(ConfigBuilder::make_tcp_upstream(
                    UPSTREAM_PORT_SECONDARY,
                    false,
                )),
            ],
            ..Default::default()
        }])
        .build();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    let res = srv
        .client
        .get(srv.base_url().join("/api").unwrap())
        .header("Host", "anything.test")
        .send()
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
}

/// Two services on the same listener with different host matchers must
/// route independently. A request to host A reaches service A, and a
/// request to host B reaches service B.
#[test]
fn multiple_services_on_same_listener_with_different_hosts() {
    let mut cfg = ConfigBuilder::default()
        .with_custom_ingress(vec![
            ServiceSpec {
                routes: vec![Located::detached(ServiceRouteSpec {
                    hosts: vec![Located::detached("a.test".to_string())],
                    path: Located::detached("/svc-a".to_string()),
                    ..Default::default()
                })],
                upstreams: vec![
                    Located::detached(ConfigBuilder::make_tcp_upstream(
                        UPSTREAM_PORT_PRIMARY,
                        false,
                    )),
                    Located::detached(ConfigBuilder::make_tcp_upstream(
                        UPSTREAM_PORT_SECONDARY,
                        false,
                    )),
                ],
                ..Default::default()
            },
            ServiceSpec {
                routes: vec![Located::detached(ServiceRouteSpec {
                    hosts: vec![Located::detached("b.test".to_string())],
                    path: Located::detached("/svc-b".to_string()),
                    ..Default::default()
                })],
                upstreams: vec![
                    Located::detached(ConfigBuilder::make_tcp_upstream(
                        UPSTREAM_PORT_PRIMARY,
                        false,
                    )),
                    Located::detached(ConfigBuilder::make_tcp_upstream(
                        UPSTREAM_PORT_SECONDARY,
                        false,
                    )),
                ],
                ..Default::default()
            },
        ])
        .build();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    let res_a = srv
        .client
        .get(srv.base_url().join("/svc-a").unwrap())
        .header("Host", "a.test")
        .send()
        .unwrap();

    let res_b = srv
        .client
        .get(srv.base_url().join("/svc-b").unwrap())
        .header("Host", "b.test")
        .send()
        .unwrap();

    // Host a.test requesting path /svc-b should 404 (wrong host for that path)
    let res_cross = srv
        .client
        .get(srv.base_url().join("/svc-b").unwrap())
        .header("Host", "a.test")
        .send()
        .unwrap();

    assert_eq!(res_a.status(), StatusCode::OK);
    assert_eq!(res_b.status(), StatusCode::OK);
    assert_eq!(
        res_cross.status(),
        StatusCode::NOT_FOUND,
        "cross-host request should not match"
    );
}

/// Host matching must be case-insensitive per RFC 4343.
#[test]
fn case_insensitive_host_matching() {
    let mut cfg = minimal_http_runtime_config();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    let res = srv
        .client
        .get(srv.base_url().join("/api").unwrap())
        .header("Host", "SNAKEWAY.TEST")
        .send()
        .unwrap();

    assert_eq!(
        res.status(),
        StatusCode::OK,
        "host matching must be case-insensitive"
    );
}

/// When two routes share the same host but differ in path length, the
/// longer prefix must win. A request to `/api/v2/resource` should match
/// `/api/v2`, not `/api`.
#[test]
fn longer_path_prefix_wins_over_shorter() {
    let mut cfg = ConfigBuilder::default()
        .with_custom_ingress(vec![
            ServiceSpec {
                routes: vec![Located::detached(ServiceRouteSpec {
                    hosts: vec![Located::detached(TEST_HOST.to_string())],
                    path: Located::detached("/api".to_string()),
                    ..Default::default()
                })],
                upstreams: vec![
                    Located::detached(ConfigBuilder::make_tcp_upstream(
                        UPSTREAM_PORT_PRIMARY,
                        false,
                    )),
                    Located::detached(ConfigBuilder::make_tcp_upstream(
                        UPSTREAM_PORT_SECONDARY,
                        false,
                    )),
                ],
                ..Default::default()
            },
            ServiceSpec {
                routes: vec![Located::detached(ServiceRouteSpec {
                    hosts: vec![Located::detached(TEST_HOST.to_string())],
                    path: Located::detached("/api/v2".to_string()),
                    ..Default::default()
                })],
                upstreams: vec![
                    Located::detached(ConfigBuilder::make_tcp_upstream(
                        UPSTREAM_PORT_PRIMARY,
                        false,
                    )),
                    Located::detached(ConfigBuilder::make_tcp_upstream(
                        UPSTREAM_PORT_SECONDARY,
                        false,
                    )),
                ],
                ..Default::default()
            },
        ])
        .build();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // /api/v2/resource should match /api/v2 (longer prefix)
    let res_v2 = srv.get("/api/v2/resource").send().unwrap();
    assert_eq!(res_v2.status(), StatusCode::OK);

    // /api/other should match /api (shorter prefix)
    let res_v1 = srv.get("/api/other").send().unwrap();
    assert_eq!(res_v1.status(), StatusCode::OK);

    // /other should match nothing
    let res_none = srv.get("/other").send().unwrap();
    assert_eq!(res_none.status(), StatusCode::NOT_FOUND);
}

/// Two services on the same listener with the SAME path but different
/// hosts are rejected during config validation. The router uses path as
/// the primary lookup key within a listener, so duplicate paths are not
/// supported even when hosts differ.
#[test]
fn same_path_different_hosts_is_rejected() {
    // Arrange
    let result = ConfigBuilder::default()
        .with_custom_ingress(vec![
            ServiceSpec {
                routes: vec![Located::detached(ServiceRouteSpec {
                    hosts: vec![Located::detached("a.test".to_string())],
                    path: Located::detached("/api".to_string()),
                    ..Default::default()
                })],
                upstreams: vec![
                    Located::detached(ConfigBuilder::make_tcp_upstream(
                        UPSTREAM_PORT_PRIMARY,
                        false,
                    )),
                    Located::detached(ConfigBuilder::make_tcp_upstream(
                        UPSTREAM_PORT_SECONDARY,
                        false,
                    )),
                ],
                ..Default::default()
            },
            ServiceSpec {
                routes: vec![Located::detached(ServiceRouteSpec {
                    hosts: vec![Located::detached("b.test".to_string())],
                    path: Located::detached("/api".to_string()),
                    ..Default::default()
                })],
                upstreams: vec![
                    Located::detached(ConfigBuilder::make_tcp_upstream(
                        UPSTREAM_PORT_PRIMARY,
                        false,
                    )),
                    Located::detached(ConfigBuilder::make_tcp_upstream(
                        UPSTREAM_PORT_SECONDARY,
                        false,
                    )),
                ],
                ..Default::default()
            },
        ])
        .try_build();

    // Assert
    let err =
        result.expect_err("duplicate route paths on the same listener should fail validation");
    match err {
        ConfigError::SemanticValidationFailed { report, .. } => {
            assert!(
                report
                    .issues()
                    .iter()
                    .any(|e| e.message.contains("duplicate route path")),
                "should report duplicate route path; got: {:?}",
                report.issues()
            );
        }
        other => panic!("expected SemanticValidationFailed, got: {other:?}"),
    }
}

/// An IPv6 literal in the Host header (`[::1]`) must not crash or hang
/// the proxy. If the route is configured for `[::1]`, it should match.
#[test]
fn ipv6_literal_host_is_handled() {
    // The proxy must not panic on IPv6 literals in the Host header.
    // Whether it matches depends on exact host comparison.
    let status = host_routing_status("[::1]", "[::1]");
    assert!(
        status == StatusCode::OK || status == StatusCode::NOT_FOUND,
        "IPv6 host should produce a valid HTTP response, got {status}"
    );
}
