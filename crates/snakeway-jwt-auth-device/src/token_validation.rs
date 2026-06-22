use crate::types::{AuthConfig, JwtClaims, JwtHeader, ValidatedToken};
use base64::Engine;
use base64::prelude::BASE64_URL_SAFE_NO_PAD;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
type HmacSha256 = Hmac<Sha256>;

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
    now: u64,
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

    if let Some(exp) = claims.exp
        && now >= exp
    {
        return Err(AuthError::TokenExpired);
    }

    if let Some(nbf) = claims.nbf
        && now < nbf
    {
        return Err(AuthError::TokenNotYetValid);
    }

    Ok(ValidatedToken { claims })
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::prelude::BASE64_URL_SAFE_NO_PAD;
    use hmac::{Hmac, KeyInit, Mac};
    use sha2::Sha256;

    const SECRET: &[u8] = b"test-secret-key-for-hmac-256!!!";
    const ISSUER: &str = "https://auth.example.com";
    const AUDIENCE: &str = "https://api.example.com";

    fn test_config() -> AuthConfig {
        AuthConfig {
            secret: SECRET.to_vec(),
            issuer: ISSUER.to_string(),
            audience: AUDIENCE.to_string(),
            user_id_claim: "sub".to_string(),
            tenant_id_claim: None,
            public_paths: vec![],
        }
    }

    fn encode_jwt(header_json: &str, payload_json: &str, secret: &[u8]) -> String {
        let header_b64 = BASE64_URL_SAFE_NO_PAD.encode(header_json.as_bytes());
        let payload_b64 = BASE64_URL_SAFE_NO_PAD.encode(payload_json.as_bytes());
        let signing_input = format!("{header_b64}.{payload_b64}");

        let mut mac = <Hmac<Sha256>>::new_from_slice(secret).unwrap();
        mac.update(signing_input.as_bytes());
        let signature = mac.finalize().into_bytes();
        let sig_b64 = BASE64_URL_SAFE_NO_PAD.encode(&signature);

        format!("{header_b64}.{payload_b64}.{sig_b64}")
    }

    fn valid_header() -> String {
        r#"{"alg":"HS256","typ":"JWT"}"#.to_string()
    }

    fn valid_payload(exp: u64) -> String {
        format!(r#"{{"sub":"user-42","iss":"{ISSUER}","aud":"{AUDIENCE}","exp":{exp}}}"#)
    }

    // -- Valid token --

    #[test]
    fn valid_token_succeeds() {
        // Arrange
        let now = 1000;
        let token = encode_jwt(&valid_header(), &valid_payload(2000), SECRET);
        let config = test_config();

        // Act
        let result = validate_token(&token, &config, now);

        // Assert
        assert!(result.is_ok());
        let validated = result.unwrap();
        assert_eq!(
            validated.claims.get_claim("sub"),
            Some("user-42".to_string())
        );
    }

    #[test]
    fn valid_token_without_exp() {
        // Arrange
        let payload = format!(r#"{{"sub":"user-1","iss":"{ISSUER}","aud":"{AUDIENCE}"}}"#);
        let token = encode_jwt(&valid_header(), &payload, SECRET);
        let config = test_config();

        // Act
        let result = validate_token(&token, &config, 999_999_999);

        // Assert
        assert!(result.is_ok());
    }

    // -- Expired token --

    #[test]
    fn expired_token_rejected() {
        // Arrange
        let token = encode_jwt(&valid_header(), &valid_payload(1000), SECRET);
        let config = test_config();

        // Act
        let result = validate_token(&token, &config, 1000);

        // Assert
        assert!(matches!(result, Err(AuthError::TokenExpired)));
    }

    #[test]
    fn token_just_before_expiry_succeeds() {
        // Arrange
        let token = encode_jwt(&valid_header(), &valid_payload(1000), SECRET);
        let config = test_config();

        // Act
        let result = validate_token(&token, &config, 999);

        // Assert
        assert!(result.is_ok());
    }

    // -- Not yet valid --

    #[test]
    fn not_yet_valid_token_rejected() {
        // Arrange
        let payload = format!(
            r#"{{"sub":"user-1","iss":"{ISSUER}","aud":"{AUDIENCE}","exp":3000,"nbf":2000}}"#
        );
        let token = encode_jwt(&valid_header(), &payload, SECRET);
        let config = test_config();

        // Act
        let result = validate_token(&token, &config, 1999);

        // Assert
        assert!(matches!(result, Err(AuthError::TokenNotYetValid)));
    }

    #[test]
    fn token_at_nbf_boundary_succeeds() {
        // Arrange
        let payload = format!(
            r#"{{"sub":"user-1","iss":"{ISSUER}","aud":"{AUDIENCE}","exp":3000,"nbf":2000}}"#
        );
        let token = encode_jwt(&valid_header(), &payload, SECRET);
        let config = test_config();

        // Act
        let result = validate_token(&token, &config, 2000);

        // Assert
        assert!(result.is_ok());
    }

    // -- Bad signature --

    #[test]
    fn wrong_secret_rejected() {
        // Arrange
        let token = encode_jwt(
            &valid_header(),
            &valid_payload(2000),
            b"wrong-secret-key!!!!!!!!!!!!!!!",
        );
        let config = test_config();

        // Act
        let result = validate_token(&token, &config, 1000);

        // Assert
        assert!(matches!(result, Err(AuthError::BadSignature)));
    }

    #[test]
    fn tampered_payload_rejected() {
        // Arrange
        let token = encode_jwt(&valid_header(), &valid_payload(2000), SECRET);
        let parts: Vec<&str> = token.splitn(3, '.').collect();
        let tampered_payload = BASE64_URL_SAFE_NO_PAD.encode(
            format!(r#"{{"sub":"admin","iss":"{ISSUER}","aud":"{AUDIENCE}","exp":2000}}"#)
                .as_bytes(),
        );
        let tampered_token = format!("{}.{}.{}", parts[0], tampered_payload, parts[2]);
        let config = test_config();

        // Act
        let result = validate_token(&tampered_token, &config, 1000);

        // Assert
        assert!(matches!(result, Err(AuthError::BadSignature)));
    }

    // -- Issuer mismatch --

    #[test]
    fn wrong_issuer_rejected() {
        // Arrange
        let payload =
            format!(r#"{{"sub":"user-1","iss":"https://evil.com","aud":"{AUDIENCE}","exp":2000}}"#);
        let token = encode_jwt(&valid_header(), &payload, SECRET);
        let config = test_config();

        // Act
        let result = validate_token(&token, &config, 1000);

        // Assert
        assert!(matches!(result, Err(AuthError::IssuerMismatch)));
    }

    #[test]
    fn missing_issuer_rejected() {
        // Arrange
        let payload = format!(r#"{{"sub":"user-1","aud":"{AUDIENCE}","exp":2000}}"#);
        let token = encode_jwt(&valid_header(), &payload, SECRET);
        let config = test_config();

        // Act
        let result = validate_token(&token, &config, 1000);

        // Assert
        assert!(matches!(result, Err(AuthError::IssuerMismatch)));
    }

    // -- Audience mismatch --

    #[test]
    fn wrong_audience_rejected() {
        // Arrange
        let payload =
            format!(r#"{{"sub":"user-1","iss":"{ISSUER}","aud":"https://other.com","exp":2000}}"#);
        let token = encode_jwt(&valid_header(), &payload, SECRET);
        let config = test_config();

        // Act
        let result = validate_token(&token, &config, 1000);

        // Assert
        assert!(matches!(result, Err(AuthError::AudienceMismatch)));
    }

    #[test]
    fn missing_audience_rejected() {
        // Arrange
        let payload = format!(r#"{{"sub":"user-1","iss":"{ISSUER}","exp":2000}}"#);
        let token = encode_jwt(&valid_header(), &payload, SECRET);
        let config = test_config();

        // Act
        let result = validate_token(&token, &config, 1000);

        // Assert
        assert!(matches!(result, Err(AuthError::AudienceMismatch)));
    }

    #[test]
    fn audience_array_matching() {
        // Arrange
        let payload = format!(
            r#"{{"sub":"user-1","iss":"{ISSUER}","aud":["{AUDIENCE}","https://other.com"],"exp":2000}}"#
        );
        let token = encode_jwt(&valid_header(), &payload, SECRET);
        let config = test_config();

        // Act
        let result = validate_token(&token, &config, 1000);

        // Assert
        assert!(result.is_ok());
    }

    // -- Unsupported algorithm --

    #[test]
    fn unsupported_algorithm_rejected() {
        // Arrange
        let header = r#"{"alg":"RS256","typ":"JWT"}"#;
        let token = encode_jwt(header, &valid_payload(2000), SECRET);
        let config = test_config();

        // Act
        let result = validate_token(&token, &config, 1000);

        // Assert
        assert!(matches!(result, Err(AuthError::UnsupportedAlgorithm(ref alg)) if alg == "RS256"));
    }

    // -- Malformed tokens --

    #[test]
    fn malformed_token_no_dots() {
        // Arrange
        let config = test_config();

        // Act
        let result = validate_token("not-a-jwt", &config, 1000);

        // Assert
        assert!(matches!(result, Err(AuthError::MalformedToken)));
    }

    #[test]
    fn malformed_token_one_dot() {
        // Arrange
        let config = test_config();

        // Act
        let result = validate_token("header.payload", &config, 1000);

        // Assert
        assert!(matches!(result, Err(AuthError::MalformedToken)));
    }

    #[test]
    fn invalid_base64_header() {
        // Arrange
        let config = test_config();

        // Act
        let result = validate_token("!!!.payload.signature", &config, 1000);

        // Assert
        assert!(matches!(result, Err(AuthError::Decode("header"))));
    }

    #[test]
    fn invalid_json_header() {
        // Arrange
        let config = test_config();
        let bad_header = BASE64_URL_SAFE_NO_PAD.encode(b"not json");
        let token = format!("{bad_header}.payload.signature");

        // Act
        let result = validate_token(&token, &config, 1000);

        // Assert
        assert!(matches!(result, Err(AuthError::Parse("header"))));
    }

    // -- AuthError status codes --

    #[test]
    fn auth_errors_return_401() {
        // Arrange
        let variants = [
            AuthError::NoToken,
            AuthError::MalformedToken,
            AuthError::BadSignature,
            AuthError::TokenExpired,
            AuthError::IssuerMismatch,
            AuthError::AudienceMismatch,
        ];

        // Act
        let statuses: Vec<u16> = variants.iter().map(|e| e.status()).collect();

        // Assert
        assert!(statuses.iter().all(|&s| s == 401));
    }

    #[test]
    fn config_error_returns_500() {
        // Arrange
        let err = AuthError::Config("test");

        // Act
        let status = err.status();

        // Assert
        assert_eq!(status, 500);
    }
}
