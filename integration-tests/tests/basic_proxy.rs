mod common;

use crate::common::spawn_upstream;
use snakeway_core::config::SnakewayConfig;
use snakeway_core::server::build_pingora_server;
use std::path::PathBuf;
use std::{thread, time::Duration};

#[test]
fn basic_proxy_works() {
    spawn_upstream();

    let cfg_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("basic.toml");

    let cfg = SnakewayConfig::from_file(cfg_path.to_str().unwrap()).expect("failed to load config");

    let server = build_pingora_server(cfg).unwrap();

    // Run Pingora in the background
    thread::spawn(move || {
        server.run_forever();
    });

    // Give it a moment to bind
    thread::sleep(Duration::from_millis(100));

    let res = reqwest::blocking::get("http://127.0.0.1:4040/").expect("request failed");

    let body = res.text().unwrap();
    assert_eq!(body, "hello world");
}
