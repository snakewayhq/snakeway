use std::sync::Once;
use std::time::Duration;

mod common;

static SERVER: Once = Once::new();
static CONFIG: &str = "basic.toml";

#[test]
fn basic_proxy_works() {
    // Arrange
    common::start_upstream();
    common::start_server(&SERVER, CONFIG);
    common::wait_for_server("127.0.0.1:4040", Duration::from_secs(2));
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();

    // Act
    let res = client
        .get("http://127.0.0.1:4040")
        .send()
        .expect("request failed");

    // Assert
    assert_eq!(res.status(), 200);
    let body = res.text().unwrap();
    assert_eq!(body, "hello world");
}
