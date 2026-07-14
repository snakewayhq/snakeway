use confval::source::Located;
use futures_util::{SinkExt, StreamExt};
use pretty_assertions::assert_eq;
use snakeway::testing_api::conf::types::{ServiceRouteSpec, ServiceSpec};
use snakeway_tests::conf::{ConfigBuilder, minimal_http_runtime_config, minimal_ws_runtime_config};
use snakeway_tests::constants::{
    ROUTE_PATH_API, ROUTE_PATH_WS, TEST_HOST, UPSTREAM_PORT_PRIMARY, UPSTREAM_PORT_SECONDARY,
};
use snakeway_tests::harness::TestServer;

#[test]
fn websocket_echo_is_proxied() {
    let mut cfg = minimal_ws_runtime_config();
    let srv = TestServer::start_ws_upstream_with_config(&mut cfg);
    let url = format!(
        "ws://{}{}",
        srv.base_url().as_str().strip_prefix("http://").unwrap(),
        ROUTE_PATH_WS
    );

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let (mut socket, _) = tokio_tungstenite::connect_async(url)
            .await
            .expect("ws connect failed");

        socket
            .send(tokio_tungstenite::tungstenite::Message::Text("ping".into()))
            .await
            .unwrap();

        let msg = socket.next().await.unwrap().unwrap();
        assert_eq!(msg.into_text().unwrap(), "ping");
    });
}

/// When `ws_max_connections` is set to 1, the proxy must reject the
/// second WebSocket connection. The upstream only handles one connection
/// at a time, so we use limit=1 to test the enforcement.
#[test]
fn websocket_max_connections_rejects_excess() {
    // Arrange
    let service = ServiceSpec {
        routes: vec![Located::detached(ServiceRouteSpec {
            hosts: vec![Located::detached(TEST_HOST.to_string())],
            path: Located::detached(ROUTE_PATH_WS.to_string()),
            enable_websocket: Located::detached(true),
            ws_max_connections: Some(Located::detached(1)),
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
    };
    let mut cfg = ConfigBuilder::default()
        .with_custom_ingress(vec![service])
        .build();
    let srv = TestServer::start_ws_upstream_with_config(&mut cfg);
    let url = format!(
        "ws://{}{}",
        srv.base_url().as_str().strip_prefix("http://").unwrap(),
        ROUTE_PATH_WS
    );

    // Act & Assert
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        // First connection should succeed.
        let (_conn1, _) = tokio_tungstenite::connect_async(&url)
            .await
            .expect("first ws connect should succeed");

        // Second connection should be rejected (proxy enforces max_connections=1).
        let result = tokio_tungstenite::connect_async(&url).await;
        assert!(
            result.is_err(),
            "second ws connection should fail when max_connections is 1"
        );
    });
}

/// Sending WebSocket upgrade headers to a route with `enable_websocket = false`
/// must return HTTP 426 Upgrade Required.
#[test]
fn websocket_upgrade_on_non_ws_route_returns_426() {
    // Arrange
    let mut cfg = minimal_http_runtime_config();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Act
    let res = srv
        .client
        .get(srv.base_url().join(ROUTE_PATH_API).unwrap())
        .header("Upgrade", "websocket")
        .header("Connection", "Upgrade")
        .header("Host", TEST_HOST)
        .send()
        .unwrap();

    // Assert
    assert_eq!(res.status().as_u16(), 426);
}
