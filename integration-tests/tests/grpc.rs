use integration_tests::harness::TestServer;
use integration_tests::harness::upstream::helloworld;
use integration_tests::harness::upstream::helloworld::HelloRequest;

use tonic::transport::{Certificate, Channel, ClientTlsConfig};

#[test]
fn grpc_unary_call_is_proxied() {
    let srv = TestServer::start_with_grpc_upstream("tls");

    let endpoint = format!(
        "https://{}",
        srv.base_url().strip_prefix("http://").unwrap()
    );

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        // Load the CA cert that signed the Pingora test cert
        let ca_pem = std::fs::read("certs/ca.pem").expect("failed to read ca.pem");

        let ca_cert = Certificate::from_pem(ca_pem);

        let tls = ClientTlsConfig::new()
            .ca_certificate(ca_cert)
            // Optional but recommended for local tests
            .domain_name("localhost");

        let channel = Channel::from_shared(endpoint)
            .expect("invalid endpoint")
            .tls_config(tls)
            .expect("tls config failed")
            .connect()
            .await
            .expect("failed to connect");

        let mut client = helloworld::greeter_client::GreeterClient::new(channel);

        let res = client
            .say_hello(tonic::Request::new(HelloRequest {
                name: "Snakeway".into(),
            }))
            .await
            .expect("grpc call failed");

        assert_eq!(res.into_inner().message, "Hello, Snakeway");
    });
}
