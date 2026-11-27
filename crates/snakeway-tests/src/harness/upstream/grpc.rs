//! gRPC upstream starter (tonic).

use crate::constants::{CERT_SERVER_KEY, CERT_SERVER_PEM};
use std::thread;

pub mod helloworld {
    tonic::include_proto!("helloworld");
}

pub fn start_grpc_upstream() -> u16 {
    use tokio_stream::wrappers::TcpListenerStream;
    use tonic::transport::{Identity, Server, ServerTlsConfig};
    use tonic::{Request, Response, Status};

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

    let (listener, port) = super::plain::bind_ephemeral();
    listener
        .set_nonblocking(true)
        .expect("failed to set nonblocking");

    thread::spawn(move || {
        // Load TLS identity (server cert + key)
        let cert = std::fs::read(CERT_SERVER_PEM).expect("failed to read server.pem");
        let key = std::fs::read(CERT_SERVER_KEY).expect("failed to read server.key");

        let identity = Identity::from_pem(cert, key);

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let listener =
                tokio::net::TcpListener::from_std(listener).expect("failed to adopt listener");
            Server::builder()
                .tls_config(ServerTlsConfig::new().identity(identity))
                .expect("failed to configure TLS")
                .add_service(GreeterServer::new(GreeterSvc))
                .serve_with_incoming(TcpListenerStream::new(listener))
                .await
                .expect("gRPC server failed");
        });
    });

    port
}
