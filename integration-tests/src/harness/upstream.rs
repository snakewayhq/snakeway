pub fn start_http_upstream(port: u16) {
    use std::io::Write;
    use std::net::TcpListener;
    use std::thread;
    use std::time::Duration;

    let addr = format!("127.0.0.1:{port}");

    thread::spawn(move || {
        let listener = TcpListener::bind(&addr).expect("failed to bind upstream");
        for stream in listener.incoming() {
            let mut stream = stream.expect("stream error");
            let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 11\r\n\r\nhello world");
        }
    });

    // tiny delay so the listener is actually ready
    thread::sleep(Duration::from_millis(25));
}

pub mod helloworld {
    tonic::include_proto!("helloworld");
}

pub fn start_grpc_upstream(port: u16) {
    use std::thread;
    use tonic::{Request, Response, Status, transport::Server};

    use helloworld::greeter_server::{Greeter, GreeterServer};
    use helloworld::{HelloReply, HelloRequest};

    #[derive(Default)]
    struct GreeterSvc;

    #[tonic::async_trait]
    impl Greeter for GreeterSvc {
        async fn say_hello(
            &self,
            req: Request<HelloRequest>,
        ) -> Result<Response<HelloReply>, Status> {
            Ok(Response::new(HelloReply {
                message: format!("Hello, {}", req.into_inner().name),
            }))
        }
    }

    thread::spawn(move || {
        let addr = format!("127.0.0.1:{port}").parse().unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            Server::builder()
                .add_service(GreeterServer::new(GreeterSvc::default()))
                .serve(addr)
                .await
                .unwrap();
        });
    });

    std::thread::sleep(std::time::Duration::from_millis(50));
}

pub fn start_ws_upstream(port: u16) {
    use futures_util::{SinkExt, StreamExt};
    use tokio::net::TcpListener;
    use tokio_tungstenite::accept_async;

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let listener = TcpListener::bind(("127.0.0.1", port)).await.unwrap();
            let (stream, _) = listener.accept().await.unwrap();

            let mut ws = accept_async(stream).await.unwrap();

            while let Some(msg) = ws.next().await {
                let msg = msg.unwrap();
                ws.send(msg).await.unwrap();
            }
        });
    });

    std::thread::sleep(std::time::Duration::from_millis(50));
}
