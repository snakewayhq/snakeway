use futures_util::{SinkExt, StreamExt};
use integration_tests::harness::TestServer;

#[test]
fn websocket_echo_is_proxied() {
    let srv = TestServer::start_with_ws_upstream("basic");

    let url = format!(
        "ws://{}/ws",
        srv.base_url().strip_prefix("http://").unwrap()
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
