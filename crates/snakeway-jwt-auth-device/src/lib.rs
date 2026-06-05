//! # JWT Auth Gateway — Snakeway WASM Device
//!
//! Validates JWT bearer tokens on incoming requests, extracts claims into
//! upstream headers, and returns synthetic responses on auth failure.
//!
//! ## Config (HCL)
//!
//! ```hcl
//! wasm_devices = [
//!   {
//!     name        = "jwt-auth"
//!     enable      = true
//!     path        = "/etc/snakeway/devices/jwt_auth_device.wasm"
//!     fail_policy = "closed"
//!     timeout_ms  = 5
//!
//!     config = {
//!       # HMAC-SHA256 shared secret (base64-encoded).
//!       secret   = "c2VjcmV0"
//!
//!       # Expected issuer claim. Tokens with a different `iss` are rejected.
//!       issuer   = "https://auth.example.com"
//!
//!       # Expected audience claim. Tokens with a different `aud` are rejected.
//!       audience = "https://api.example.com"
//!
//!       # Claim mapped to the X-User-Id upstream header. Default: "sub".
//!       user_id_claim = "sub"
//!
//!       # Claim mapped to the X-Tenant-Id upstream header. Optional.
//!       # Omit to skip tenant header injection.
//!       tenant_id_claim = "tenant_id"
//!
//!       # Comma-separated paths that bypass authentication entirely.
//!       # Supports exact matches only.
//!       public_paths = "/health,/ready,/.well-known/openid-configuration"
//!     }
//!   }
//! ]
//! ```
//!
//! ## Behavior
//!
//! **on-request:** Validates the JWT and injects upstream headers on success.
//!   - 401 if no `Authorization: Bearer <token>` header is present.
//!   - 401 if the token signature is invalid.
//!   - 401 if `iss` or `aud` claims do not match config.
//!   - 401 if the token is expired (`exp`) or not yet valid (`nbf`).
//!   - On success: continues with a patch that adds `X-User-Id` (and optionally
//!     `X-Tenant-Id`) and removes the `Authorization` header before proxying.
//!   - Paths listed in `public_paths` bypass validation entirely.
//!
//! **All other hooks:** Passthrough (no-op).
use wit_bindgen::generate;

generate!({
    path: "../snakeway-wit/wit/",
    world: "device",
});

use exports::snakeway::device::policy::Guest;

use crate::snakeway::device::host;
use crate::snakeway::device::types::{
    Action, BodyAction, BodyChunk, BodyResult, Header, HeaderOp, Request, RequestPatch,
    RequestResult, Response, ResponseResult, SyntheticResponse,
};

use base64::prelude::*;
use hmac::{Hmac, KeyInit, Mac};
use serde::Deserialize;
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

struct AuthConfig {
    secret: Vec<u8>,
    issuer: String,
    audience: String,
    user_id_claim: String,
    tenant_id_claim: Option<String>,
    public_paths: Vec<String>,
}

impl AuthConfig {
    fn from_host() -> Result<Self, AuthError> {
        let secret_b64 = host::config_get("secret")
            .ok_or(AuthError::Config("missing required config key: secret"))?;

        let secret = BASE64_STANDARD
            .decode(secret_b64.as_bytes())
            .map_err(|_| AuthError::Config("secret is not valid base64"))?;

        let issuer = host::config_get("issuer")
            .ok_or(AuthError::Config("missing required config key: issuer"))?;

        let audience = host::config_get("audience")
            .ok_or(AuthError::Config("missing required config key: audience"))?;

        let user_id_claim = host::config_get("user_id_claim").unwrap_or_else(|| "sub".to_string());

        let tenant_id_claim = host::config_get("tenant_id_claim");

        let public_paths = host::config_get("public_paths")
            .map(|s| s.split(',').map(|p| p.trim().to_string()).collect())
            .unwrap_or_default();

        Ok(Self {
            secret,
            issuer,
            audience,
            user_id_claim,
            tenant_id_claim,
            public_paths,
        })
    }
}

// ---------------------------------------------------------------------------
// JWT types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct JwtHeader {
    alg: String,
    #[allow(dead_code)]
    typ: Option<String>,
}

#[derive(Deserialize)]
struct JwtClaims {
    #[serde(default)]
    iss: Option<String>,

    #[serde(default)]
    aud: Option<Audience>,

    #[serde(default)]
    sub: Option<String>,

    #[serde(default)]
    exp: Option<u64>,

    #[serde(default)]
    nbf: Option<u64>,

    #[serde(flatten)]
    extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Audience {
    Single(String),
    Multiple(Vec<String>),
}

impl Audience {
    fn contains(&self, expected: &str) -> bool {
        match self {
            Audience::Single(s) => s == expected,
            Audience::Multiple(v) => v.iter().any(|s| s == expected),
        }
    }
}

impl JwtClaims {
    fn get_claim(&self, name: &str) -> Option<String> {
        match name {
            "sub" => self.sub.clone(),
            "iss" => self.iss.clone(),
            other => self.extra.get(other).and_then(|v| match v {
                serde_json::Value::String(s) => Some(s.clone()),
                serde_json::Value::Number(n) => Some(n.to_string()),
                serde_json::Value::Bool(b) => Some(b.to_string()),
                _ => None,
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// JWT parsing and validation
// ---------------------------------------------------------------------------

struct ValidatedToken {
    claims: JwtClaims,
}

#[derive(Debug)]
enum AuthError {
    NoToken,
    MalformedToken,
    UnsupportedAlgorithm(String),
    Decode(&'static str),
    Parse(&'static str),
    BadSignature,
    IssuerMismatch,
    AudienceMismatch,
    TokenExpired,
    TokenNotYetValid,
    Config(&'static str),
}

impl AuthError {
    fn status(&self) -> u16 {
        match self {
            AuthError::Config(_) => 500,
            _ => 401,
        }
    }

    fn message(&self) -> &str {
        match self {
            AuthError::NoToken => "missing or malformed authorization header",
            AuthError::MalformedToken => "malformed jwt token",
            AuthError::UnsupportedAlgorithm(_) => "unsupported signing algorithm",
            AuthError::Decode(_) => "token decoding failed",
            AuthError::Parse(_) => "token parsing failed",
            AuthError::BadSignature => "invalid token signature",
            AuthError::IssuerMismatch => "token issuer not accepted",
            AuthError::AudienceMismatch => "token audience not accepted",
            AuthError::TokenExpired => "token has expired",
            AuthError::TokenNotYetValid => "token is not yet valid",
            AuthError::Config(_) => "authentication service misconfigured",
        }
    }

    fn log_message(&self) -> String {
        match self {
            AuthError::Config(detail) => format!("config error: {detail}"),
            AuthError::UnsupportedAlgorithm(alg) => format!("unsupported algorithm: {alg}"),
            AuthError::Decode(part) => format!("base64 decode failed: {part}"),
            AuthError::Parse(part) => format!("json parse failed: {part}"),
            other => other.message().to_string(),
        }
    }
}

fn base64url_decode(input: &str) -> Result<Vec<u8>, &'static str> {
    BASE64_URL_SAFE_NO_PAD
        .decode(input.as_bytes())
        .map_err(|_| "base64url decode failed")
}

fn validate_token(raw_token: &str, config: &AuthConfig) -> Result<ValidatedToken, AuthError> {
    let parts: Vec<&str> = raw_token.splitn(3, '.').collect();
    if parts.len() != 3 {
        return Err(AuthError::MalformedToken);
    }

    let (header_b64, payload_b64, signature_b64) = (parts[0], parts[1], parts[2]);

    let header_bytes = base64url_decode(header_b64).map_err(|_| AuthError::Decode("header"))?;
    let header: JwtHeader =
        serde_json::from_slice(&header_bytes).map_err(|_| AuthError::Parse("header"))?;

    if header.alg != "HS256" {
        return Err(AuthError::UnsupportedAlgorithm(header.alg));
    }

    let signing_input = format!("{header_b64}.{payload_b64}");
    let signature = base64url_decode(signature_b64).map_err(|_| AuthError::Decode("signature"))?;

    let mut mac = HmacSha256::new_from_slice(&config.secret)
        .map_err(|_| AuthError::Config("secret key invalid for HMAC"))?;
    mac.update(signing_input.as_bytes());

    mac.verify_slice(&signature)
        .map_err(|_| AuthError::BadSignature)?;

    let payload_bytes = base64url_decode(payload_b64).map_err(|_| AuthError::Decode("payload"))?;
    let claims: JwtClaims =
        serde_json::from_slice(&payload_bytes).map_err(|_| AuthError::Parse("payload"))?;

    match &claims.iss {
        Some(iss) if iss == &config.issuer => {}
        _ => return Err(AuthError::IssuerMismatch),
    }

    match &claims.aud {
        Some(aud) if aud.contains(&config.audience) => {}
        _ => return Err(AuthError::AudienceMismatch),
    }

    let now = host::epoch_secs();

    if let Some(exp) = claims.exp {
        if now >= exp {
            return Err(AuthError::TokenExpired);
        }
    }

    if let Some(nbf) = claims.nbf {
        if now < nbf {
            return Err(AuthError::TokenNotYetValid);
        }
    }

    Ok(ValidatedToken { claims })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn extract_bearer_token(auth_value: &str) -> Option<&str> {
    let trimmed = auth_value.trim();
    if trimmed.len() > 7 && trimmed[..7].eq_ignore_ascii_case("bearer ") {
        Some(trimmed[7..].trim())
    } else {
        None
    }
}

fn error_body(error: &str) -> Vec<u8> {
    let escaped = error.replace('\\', "\\\\").replace('"', "\\\"");
    format!(r#"{{"error":"{escaped}"}}"#).into_bytes()
}

fn error_response(err: &AuthError) -> Action {
    Action::Respond(SyntheticResponse {
        status: err.status(),
        headers: vec![
            Header {
                name: "content-type".to_string(),
                value: "application/json".to_string(),
            },
            Header {
                name: "cache-control".to_string(),
                value: "no-store".to_string(),
            },
        ],
        body: error_body(err.message()),
    })
}

fn success_patch(token: &ValidatedToken, config: &AuthConfig) -> Option<RequestPatch> {
    let mut ops = Vec::new();

    if let Some(user_id) = token.claims.get_claim(&config.user_id_claim) {
        ops.push(HeaderOp::Set(Header {
            name: "x-user-id".to_string(),
            value: user_id,
        }));
    }

    if let Some(ref tenant_claim) = config.tenant_id_claim {
        if let Some(tenant_id) = token.claims.get_claim(tenant_claim) {
            ops.push(HeaderOp::Set(Header {
                name: "x-tenant-id".to_string(),
                value: tenant_id,
            }));
        }
    }

    ops.push(HeaderOp::Remove("authorization".to_string()));

    Some(RequestPatch {
        set_route_path: None,
        set_upstream_path: None,
        ops,
    })
}

// ---------------------------------------------------------------------------
// Device implementation
// ---------------------------------------------------------------------------

struct JwtAuthDevice;

impl Guest for JwtAuthDevice {
    fn on_request(req: Request) -> RequestResult {
        let config = match AuthConfig::from_host() {
            Ok(c) => c,
            Err(e) => {
                host::log(4, &format!("jwt-auth config error: {}", e.log_message()));
                host::metric_increment("auth_config_errors", 1);
                return RequestResult {
                    action: error_response(&e),
                    patch: None,
                };
            }
        };

        if config.public_paths.iter().any(|p| p == &req.route_path) {
            host::log(0, &format!("public path bypass: {}", req.route_path));
            return RequestResult {
                action: Action::Continue,
                patch: None,
            };
        }

        let auth_header = req
            .headers
            .iter()
            .find(|h| h.name.eq_ignore_ascii_case("authorization"));

        let raw_token = match auth_header.and_then(|h| extract_bearer_token(&h.value)) {
            Some(t) => t,
            None => {
                host::log(
                    2,
                    &format!("no bearer token: {} {}", req.method, req.route_path),
                );
                host::metric_increment("auth_rejected", 1);
                return RequestResult {
                    action: error_response(&AuthError::NoToken),
                    patch: None,
                };
            }
        };

        match validate_token(raw_token, &config) {
            Ok(token) => {
                let user_id = token
                    .claims
                    .get_claim(&config.user_id_claim)
                    .unwrap_or_else(|| "unknown".to_string());

                host::log(
                    1,
                    &format!(
                        "auth ok: user={} {} {}",
                        user_id, req.method, req.route_path
                    ),
                );
                host::metric_increment("auth_accepted", 1);

                RequestResult {
                    action: Action::Continue,
                    patch: success_patch(&token, &config),
                }
            }
            Err(e) => {
                host::log(
                    3,
                    &format!(
                        "auth rejected: {} — {} {}",
                        e.log_message(),
                        req.method,
                        req.route_path
                    ),
                );
                host::metric_increment("auth_rejected", 1);

                RequestResult {
                    action: error_response(&e),
                    patch: None,
                }
            }
        }
    }

    fn on_stream_request_body(_req: Request, _chunk: Option<BodyChunk>) -> BodyResult {
        BodyResult {
            action: BodyAction::Passthrough,
        }
    }

    fn before_proxy(_req: Request) -> RequestResult {
        RequestResult {
            action: Action::Continue,
            patch: None,
        }
    }

    fn after_proxy(_resp: Response) -> ResponseResult {
        ResponseResult {
            action: Action::Continue,
            patch: None,
        }
    }

    fn on_stream_response_body(_resp: Response, _chunk: Option<BodyChunk>) -> BodyResult {
        BodyResult {
            action: BodyAction::Passthrough,
        }
    }

    fn on_response(_resp: Response) -> ResponseResult {
        ResponseResult {
            action: Action::Continue,
            patch: None,
        }
    }
}

export!(JwtAuthDevice);
