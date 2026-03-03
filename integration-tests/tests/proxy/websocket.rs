use futures_util::{SinkExt, StreamExt};
use integration_tests::conf::minimal_ws_runtime_config;
use integration_tests::constants::ROUTE_PATH_WS;
use integration_tests::harness::TestServer;
use pretty_assertions::assert_eq;

#[test]
fn websocket_echo_is_proxied() {
    let mut cfg = minimal_ws_runtime_config();
    let srv = TestServer::start_ws_upstream_with_config(&mut cfg);
    let url = format!(
        "ws://{}{}",
        srv.base_url().strip_prefix("http://").unwrap(),
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
