mod common;

use snakeway_core::config::SnakewayConfig;
use snakeway_core::server::build_pingora_server;

use std::path::PathBuf;
use std::{thread, time::Duration};

fn load_static_config() -> SnakewayConfig {
    let cfg_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("static.toml");

    SnakewayConfig::from_file(cfg_path.to_str().unwrap())
        .expect("failed to load static.toml config")
}

fn start_server(cfg: SnakewayConfig) {
    let server = build_pingora_server(cfg).expect("failed to build pingora server");

    thread::spawn(move || {
        server.run_forever();
    });

    // Give Pingora time to bind
    thread::sleep(Duration::from_millis(150));
}

#[test]
fn serves_index_html_from_static_dir() {
    // Arrange
    let cfg = load_static_config();
    start_server(cfg);

    // Act
    let res = reqwest::blocking::get("http://127.0.0.1:4041/")
        .expect("static request failed");

    let status = res.status();
    let body = res.text().expect("failed to read response body");

    // Assert
    assert_eq!(status, 200);
    assert!(
        body.contains("Snakeway"),
        "unexpected response body: {body}"
    );
}

#[test]
fn static_route_does_not_require_upstream() {
    // Arrange
    // NOTE: intentionally NOT spawning upstream
    let cfg = load_static_config();
    start_server(cfg);

    // Act
    let res = reqwest::blocking::get("http://127.0.0.1:4041/")
        .expect("static request failed");

    // Assert
    assert_eq!(res.status(), 200);
}

#[test]
fn proxy_route_still_works_when_static_is_enabled() {
    // Arrange
    common::spawn_upstream();

    let cfg = load_static_config();
    start_server(cfg);

    // Act
    let res = reqwest::blocking::get("http://127.0.0.1:4041/api")
        .expect("proxy request failed");

    let status = res.status();
    let body = res.text().expect("failed to read response body");

    // Assert
    assert_eq!(status, 200);
    assert_eq!(body, "hello world");
}
