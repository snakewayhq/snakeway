//! End-to-end tests for the real `snakeway-jwt-auth-device` running inside the
//! proxy. These exercise the full chain (device + host pipeline + C1 header
//! writeback) that host unit tests cannot reach, because the device's request
//! handling is wasm-gated.
//!
//! Each test mints a real HS256 token, configures the device with the matching
//! secret, and asserts against the echo-headers upstream what the upstream
//! actually received.

use base64::Engine;
use base64::prelude::{BASE64_STANDARD, BASE64_URL_SAFE_NO_PAD};
use hmac::{Hmac, KeyInit, Mac};
use pretty_assertions::assert_eq;
use reqwest::StatusCode;
use sha2::Sha256;
use snakeway_tests::conf::ConfigBuilder;
use snakeway_tests::device::make_jwt_device;
use snakeway_tests::harness::TestServer;
use std::collections::HashMap;

/// 32-byte HMAC secret (the device rejects anything shorter).
const SECRET: &[u8] = b"integration-jwt-hmac-secret-0001";
const ISSUER: &str = "https://auth.example.com";
const AUDIENCE: &str = "https://api.example.com";
/// Far-future expiry so the token stays valid against real wall-clock time.
const FUTURE_EXP: u64 = 4_102_444_800; // 2100-01-01

/// Sign an HS256 JWT with the given header and payload JSON.
fn mint_with_header(header_json: &str, payload_json: &str) -> String {
    let header_b64 = BASE64_URL_SAFE_NO_PAD.encode(header_json.as_bytes());
    let payload_b64 = BASE64_URL_SAFE_NO_PAD.encode(payload_json.as_bytes());
    let signing_input = format!("{header_b64}.{payload_b64}");
    let mut mac = <Hmac<Sha256>>::new_from_slice(SECRET).unwrap();
    mac.update(signing_input.as_bytes());
    let sig_b64 = BASE64_URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());
    format!("{signing_input}.{sig_b64}")
}

/// Sign an HS256 JWT with the default `{"alg":"HS256","typ":"JWT"}` header.
fn mint(payload_json: &str) -> String {
    mint_with_header(r#"{"alg":"HS256","typ":"JWT"}"#, payload_json)
}

fn valid_payload() -> String {
    format!(
        r#"{{"sub":"user-42","tenant_id":"acme","iss":"{ISSUER}","aud":"{AUDIENCE}","exp":{FUTURE_EXP}}}"#
    )
}

fn valid_token() -> String {
    mint(&valid_payload())
}

/// Current wall-clock time in whole seconds, matching the device's `now`.
fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Device config with the matching secret. `public_paths` is opt-in per test.
fn jwt_config() -> HashMap<String, String> {
    HashMap::from([
        ("secret".to_string(), BASE64_STANDARD.encode(SECRET)),
        ("issuer".to_string(), ISSUER.to_string()),
        ("audience".to_string(), AUDIENCE.to_string()),
        ("user_id_claim".to_string(), "sub".to_string()),
        ("tenant_id_claim".to_string(), "tenant_id".to_string()),
    ])
}

/// Parse the echo-headers upstream response body and look up a header by name.
fn echoed_header(body: &str, name: &str) -> Option<String> {
    let json: serde_json::Value = serde_json::from_str(body)
        .unwrap_or_else(|e| panic!("upstream response is not valid JSON: {e}\nbody: {body}"));
    json.get(name).and_then(|v| v.as_str()).map(String::from)
}

/// A valid token: the device injects identity headers from the claims and strips
/// the Authorization header before the request reaches the upstream.
#[test]
fn jwt_injects_identity_and_strips_authorization() {
    // Arrange
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_wasm_device(make_jwt_device(jwt_config()))
        .build();
    let srv = TestServer::start_http_upstream_that_echoes_headers_with_config(&mut cfg);

    // Act
    let res = srv
        .get("/api")
        .header("authorization", format!("Bearer {}", valid_token()))
        .send()
        .expect("request failed");

    // Assert
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.text().unwrap();
    assert_eq!(
        echoed_header(&body, "x-user-id").as_deref(),
        Some("user-42")
    );
    assert_eq!(echoed_header(&body, "x-tenant-id").as_deref(), Some("acme"));
    assert_eq!(echoed_header(&body, "authorization"), None);
}

/// A client-supplied X-User-Id must not survive: the device overrides it with the
/// token-derived identity.
#[test]
fn jwt_overrides_client_supplied_identity_header() {
    // Arrange
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_wasm_device(make_jwt_device(jwt_config()))
        .build();
    let srv = TestServer::start_http_upstream_that_echoes_headers_with_config(&mut cfg);

    // Act
    let res = srv
        .get("/api")
        .header("authorization", format!("Bearer {}", valid_token()))
        .header("x-user-id", "attacker")
        .header("x-tenant-id", "attacker-tenant")
        .send()
        .expect("request failed");

    // Assert
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.text().unwrap();
    assert_eq!(
        echoed_header(&body, "x-user-id").as_deref(),
        Some("user-42")
    );
    assert_eq!(echoed_header(&body, "x-tenant-id").as_deref(), Some("acme"));
}

/// A request with no bearer token is rejected with 401 and never proxied.
#[test]
fn jwt_rejects_request_without_token() {
    // Arrange
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_wasm_device(make_jwt_device(jwt_config()))
        .build();
    let srv = TestServer::start_http_upstream_that_echoes_headers_with_config(&mut cfg);

    // Act
    let res = srv.get("/api").send().expect("request failed");

    // Assert
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(res.text().unwrap(), r#"{"error":"unauthorized"}"#);
}

/// A token whose signature does not match the configured secret is rejected.
#[test]
fn jwt_rejects_token_with_bad_signature() {
    // Arrange
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_wasm_device(make_jwt_device(jwt_config()))
        .build();
    let srv = TestServer::start_http_upstream_that_echoes_headers_with_config(&mut cfg);
    let tampered = format!("{}tampered", valid_token());

    // Act
    let res = srv
        .get("/api")
        .header("authorization", format!("Bearer {tampered}"))
        .send()
        .expect("request failed");

    // Assert
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

/// On a public-path bypass the device still strips client-supplied identity
/// headers, so a request to a public path cannot spoof X-User-Id.
#[test]
fn jwt_public_path_strips_client_identity_header() {
    // Arrange
    let mut config = jwt_config();
    config.insert("public_paths".to_string(), "/api".to_string());
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_wasm_device(make_jwt_device(config))
        .build();
    let srv = TestServer::start_http_upstream_that_echoes_headers_with_config(&mut cfg);

    // Act
    let res = srv
        .get("/api")
        .header("x-user-id", "attacker")
        .header("x-tenant-id", "attacker-tenant")
        .send()
        .expect("request failed");

    // Assert
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.text().unwrap();
    assert_eq!(echoed_header(&body, "x-user-id"), None);
    assert_eq!(echoed_header(&body, "x-tenant-id"), None);
}

/// With `token_type` configured, a token whose `typ` does not match is rejected.
/// This also proves the config key is read end-to-end.
#[test]
fn jwt_rejects_mismatched_token_type() {
    // Arrange
    let mut config = jwt_config();
    config.insert("token_type".to_string(), "at+jwt".to_string());
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_wasm_device(make_jwt_device(config))
        .build();
    let srv = TestServer::start_http_upstream_that_echoes_headers_with_config(&mut cfg);
    let token = mint_with_header(r#"{"alg":"HS256","typ":"JWT"}"#, &valid_payload());

    // Act
    let res = srv
        .get("/api")
        .header("authorization", format!("Bearer {token}"))
        .send()
        .expect("request failed");

    // Assert
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

/// With a large `clock_skew_leeway_seconds`, a token that expired slightly in the
/// past is still accepted. This proves the leeway config key is read end-to-end.
#[test]
fn jwt_leeway_accepts_recently_expired_token() {
    // Arrange
    let mut config = jwt_config();
    config.insert("clock_skew_leeway_seconds".to_string(), "3600".to_string());
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_wasm_device(make_jwt_device(config))
        .build();
    let srv = TestServer::start_http_upstream_that_echoes_headers_with_config(&mut cfg);
    let exp = now_secs().saturating_sub(30);
    let token = mint(&format!(
        r#"{{"sub":"user-42","tenant_id":"acme","iss":"{ISSUER}","aud":"{AUDIENCE}","exp":{exp}}}"#
    ));

    // Act
    let res = srv
        .get("/api")
        .header("authorization", format!("Bearer {token}"))
        .send()
        .expect("request failed");

    // Assert
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.text().unwrap();
    assert_eq!(
        echoed_header(&body, "x-user-id").as_deref(),
        Some("user-42")
    );
}

/// A token whose `jti` is on the configured denylist is rejected. This also
/// proves the `revoked_jti` config key is read end-to-end.
#[test]
fn jwt_rejects_revoked_token() {
    // Arrange
    let mut config = jwt_config();
    config.insert("revoked_jti".to_string(), "revoked-1".to_string());
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_wasm_device(make_jwt_device(config))
        .build();
    let srv = TestServer::start_http_upstream_that_echoes_headers_with_config(&mut cfg);
    let token = mint(&format!(
        r#"{{"sub":"user-42","iss":"{ISSUER}","aud":"{AUDIENCE}","exp":{FUTURE_EXP},"jti":"revoked-1"}}"#
    ));

    // Act
    let res = srv
        .get("/api")
        .header("authorization", format!("Bearer {token}"))
        .send()
        .expect("request failed");

    // Assert
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}
