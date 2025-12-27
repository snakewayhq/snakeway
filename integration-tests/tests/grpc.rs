use integration_tests::harness::TestServer;
use integration_tests::harness::upstream::helloworld;
use integration_tests::harness::upstream::helloworld::HelloRequest;

#[test]
fn grpc_unary_call_is_proxied() {
    let srv = TestServer::start_with_grpc_upstream("basic");

    let endpoint = format!("http://{}", srv.base_url().strip_prefix("http://").unwrap());

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let mut client = helloworld::greeter_client::GreeterClient::connect(endpoint)
            .await
            .expect("failed to connect");

        let res = client
            .say_hello(tonic::Request::new(HelloRequest {
                name: "Snakeway".into(),
            }))
            .await
            .expect("grpc call failed");

        assert_eq!(res.into_inner().message, "Hello, Snakeway");
    });
}
