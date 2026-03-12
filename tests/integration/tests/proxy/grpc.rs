use integration::conf::minimal_grpc_runtime_config;
use integration::constants::{CERT_ORIGIN_CA_PEM, TEST_HOST};
use integration::harness::TestServer;
use integration::harness::upstream::helloworld;
use integration::harness::upstream::helloworld::HelloRequest;
use pretty_assertions::assert_eq;
use tonic::transport::{Certificate, Channel, ClientTlsConfig};

#[test]
fn grpc_unary_call_is_proxied() {
    let mut cfg = minimal_grpc_runtime_config();
    let srv = TestServer::start_grpc_upstream_with_config(&mut cfg);

    let endpoint = format!(
        "https://{}",
        srv.base_url().strip_prefix("http://").unwrap()
    );

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        // Load the CA cert that signed the Pingora test cert
        let ca_pem = std::fs::read(CERT_ORIGIN_CA_PEM).expect("failed to read ca.pem");

        let ca_cert = Certificate::from_pem(ca_pem);

        let tls = ClientTlsConfig::new()
            .ca_certificate(ca_cert)
            .domain_name(TEST_HOST);

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
