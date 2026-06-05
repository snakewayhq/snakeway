use crate::bindings::host;
use crate::config::AuthConfig;
use crate::types::HmacSha256;
use crate::types::{JwtClaims, JwtHeader};
use base64::Engine;
use base64::prelude::BASE64_URL_SAFE_NO_PAD;
use hmac::KeyInit;
use hmac::Mac;

pub(crate) struct ValidatedToken {
    pub(crate) claims: JwtClaims,
}

#[derive(Debug)]
pub(crate) enum AuthError {
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
    pub(crate) fn status(&self) -> u16 {
        match self {
            AuthError::Config(_) => 500,
            _ => 401,
        }
    }

    pub(crate) fn message(&self) -> &str {
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

    pub(crate) fn log_message(&self) -> String {
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

pub(crate) fn validate_token(
    raw_token: &str,
    config: &AuthConfig,
) -> Result<ValidatedToken, AuthError> {
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
