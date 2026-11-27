//! WebSocket echo upstream starter.

use std::thread;

pub fn start_ws_upstream() -> u16 {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::accept_async;

    let (listener, port) = super::plain::bind_ephemeral();
    listener
        .set_nonblocking(true)
        .expect("failed to set nonblocking");

    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let listener =
                tokio::net::TcpListener::from_std(listener).expect("failed to adopt listener");

            loop {
                let (stream, _) = listener.accept().await.unwrap();

                tokio::spawn(async move {
                    let mut ws = accept_async(stream).await.unwrap();

                    while let Some(Ok(msg)) = ws.next().await {
                        if ws.send(msg).await.is_err() {
                            break;
                        }
                    }
                });
            }
        });
    });

    port
}
