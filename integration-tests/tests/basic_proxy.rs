mod common;

use snakeway_core::server::build_pingora_server;
use std::{thread, time::Duration};

#[test]
fn basic_proxy_works() {
    // Arrange
    common::start_upstream();
    let cfg = common::load_config("basic.toml");
    let server = build_pingora_server(cfg).unwrap();
    thread::spawn(move || {
        server.run_forever();
    });
    thread::sleep(Duration::from_millis(100)); // Give it a moment to bind

    // Act
    let res = reqwest::blocking::get("http://127.0.0.1:4040/").expect("request failed");

    // Assert
    let body = res.text().unwrap();
    assert_eq!(body, "hello world");
}
