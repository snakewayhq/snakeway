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
    MissingExpiry,
    TokenNotYetValid,
    MissingUserId,
    InvalidClaimValue,
    TypeMismatch,
    TokenRevoked,
    MissingJti,
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
            AuthError::MissingExpiry => "token has no expiry",
            AuthError::TokenNotYetValid => "token is not yet valid",
            AuthError::MissingUserId => "token missing required identity claim",
            AuthError::InvalidClaimValue => "token identity claim is not usable",
            AuthError::TypeMismatch => "token type not accepted",
            AuthError::TokenRevoked => "token has been revoked",
            AuthError::MissingJti => "token missing required id claim",
            AuthError::Config(_) => "authentication service misconfigured",
        }
    }

    pub(crate) fn log_message(&self) -> String {
        match self {
            AuthError::Config(detail) => format!("config error: {detail}"),
            AuthError::UnsupportedAlgorithm(alg) => format!("unsupported algorithm: {alg:?}"),
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

/// Minimum HS256 secret length. RFC 8725 §3.5 / RFC 7518 §3.2 require a key at
/// least as long as the HMAC-SHA256 output (256 bits = 32 bytes).
pub(crate) const MIN_SECRET_BYTES: usize = 32;

/// Reject secrets shorter than [`MIN_SECRET_BYTES`]. The `hmac` crate accepts a
/// key of any length (including zero), so without this guard an empty or short,
/// brute-forceable secret would be used silently. Enforced at config load so the
/// device fails closed rather than running with a forgeable key.
pub(crate) fn validate_secret(secret: &[u8]) -> Result<(), AuthError> {
    if secret.len() < MIN_SECRET_BYTES {
        return Err(AuthError::Config("secret must be at least 32 bytes"));
    }
    Ok(())
}

/// Normalize a JWT `typ` value for comparison: `typ` is case-insensitive and the
/// `application/` media-type prefix may be omitted (RFC 7519 §5.1).
fn normalize_typ(value: &str) -> String {
    let without_prefix =
        if value.len() >= 12 && value.as_bytes()[..12].eq_ignore_ascii_case(b"application/") {
            &value[12..]
        } else {
            value
        };
    without_prefix.to_ascii_lowercase()
}

/// Parse the optional `clock_skew_leeway_seconds` config value. Absent means 0.
pub(crate) fn parse_leeway_seconds(raw: Option<&str>) -> Result<u64, AuthError> {
    match raw {
        Some(s) => s.trim().parse::<u64>().map_err(|_| {
            AuthError::Config("clock_skew_leeway_seconds must be a non-negative integer")
        }),
        None => Ok(0),
    }
}

/// Identity claim values are placed into upstream headers. Reject empty values
/// and values that contain control characters, which are not valid in a header
/// value and could enable header injection if the host did not also reject them.
fn is_safe_header_value(value: &str) -> bool {
    !value.is_empty() && !value.chars().any(char::is_control)
}

pub(crate) fn validate_token(
    raw_token: &str,
    config: &AuthConfig,
    now: u64,
) -> Result<ValidatedToken, AuthError> {
    // A zero clock means the host could not read time (`epoch_secs` returns 0 on
    // error). Fail closed rather than treating every token as not-yet-expired.
    if now == 0 {
        return Err(AuthError::Config("host clock unavailable"));
    }

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

    // Explicit typing (RFC 8725 §3.11): when an expected type is configured,
    // reject tokens of a different (or absent) `typ` to prevent cross-JWT
    // substitution when the issuer signs multiple token kinds with one secret.
    if let Some(expected) = &config.token_type {
        match &header.typ {
            Some(typ) if normalize_typ(typ) == normalize_typ(expected) => {}
            _ => return Err(AuthError::TypeMismatch),
        }
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

    // Expiry is mandatory. A token with no `exp` would otherwise be accepted
    // forever, so reject when it is absent (RFC 8725 §3.7). The configured clock
    // skew leeway widens both bounds to absorb drift between issuer and proxy.
    let leeway = config.clock_skew_leeway_seconds;
    let exp = claims.exp.ok_or(AuthError::MissingExpiry)?;
    if now >= exp.saturating_add(leeway) {
        return Err(AuthError::TokenExpired);
    }

    if let Some(nbf) = claims.nbf
        && now.saturating_add(leeway) < nbf
    {
        return Err(AuthError::TokenNotYetValid);
    }

    // Identity must be resolvable from the configured claim and usable as a header
    // value; the device asserts it to the upstream as X-User-Id. Reject when it is
    // missing or non-scalar, and when it is empty or contains control characters.
    match claims.get_claim(&config.user_id_claim) {
        Some(user_id) if is_safe_header_value(&user_id) => {}
        Some(_) => return Err(AuthError::InvalidClaimValue),
        None => return Err(AuthError::MissingUserId),
    }

    // The tenant claim, when configured and present, is also asserted as a header,
    // so it must be a usable value too.
    if let Some(tenant_claim) = &config.tenant_id_claim
        && let Some(tenant_id) = claims.get_claim(tenant_claim)
        && !is_safe_header_value(&tenant_id)
    {
        return Err(AuthError::InvalidClaimValue);
    }

    // Revocation: when a denylist is configured, the token must carry a `jti`
    // that is not on it. An enabled denylist makes `jti` mandatory so that every
    // accepted token is revocable.
    if !config.revoked_jti.is_empty() {
        match &claims.jti {
            Some(jti) if config.revoked_jti.contains(jti) => return Err(AuthError::TokenRevoked),
            Some(_) => {}
            None => return Err(AuthError::MissingJti),
        }
    }

    Ok(ValidatedToken { claims })
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::prelude::BASE64_URL_SAFE_NO_PAD;
    use hmac::{Hmac, KeyInit, Mac};
    use sha2::Sha256;
    use std::collections::HashSet;

    const SECRET: &[u8] = b"test-secret-key-for-hmac-sha256!";
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
            token_type: None,
            clock_skew_leeway_seconds: 0,
            revoked_jti: HashSet::new(),
        }
    }

    fn encode_jwt(header_json: &str, payload_json: &str, secret: &[u8]) -> String {
        let header_b64 = BASE64_URL_SAFE_NO_PAD.encode(header_json.as_bytes());
        let payload_b64 = BASE64_URL_SAFE_NO_PAD.encode(payload_json.as_bytes());
        let signing_input = format!("{header_b64}.{payload_b64}");

        let mut mac = <Hmac<Sha256>>::new_from_slice(secret).unwrap();
        mac.update(signing_input.as_bytes());
        let signature = mac.finalize().into_bytes();
        let sig_b64 = BASE64_URL_SAFE_NO_PAD.encode(signature);

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
    fn token_without_exp_is_rejected() {
        // Arrange
        let payload = format!(r#"{{"sub":"user-1","iss":"{ISSUER}","aud":"{AUDIENCE}"}}"#);
        let token = encode_jwt(&valid_header(), &payload, SECRET);
        let config = test_config();

        // Act
        let result = validate_token(&token, &config, 999_999_999);

        // Assert
        assert!(matches!(result, Err(AuthError::MissingExpiry)));
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

    // -- Missing identity claim --

    #[test]
    fn token_missing_user_id_claim_is_rejected() {
        // Arrange
        let payload = format!(r#"{{"iss":"{ISSUER}","aud":"{AUDIENCE}","exp":2000}}"#);
        let token = encode_jwt(&valid_header(), &payload, SECRET);
        let config = test_config();

        // Act
        let result = validate_token(&token, &config, 1000);

        // Assert
        assert!(matches!(result, Err(AuthError::MissingUserId)));
    }

    #[test]
    fn token_with_non_scalar_user_id_claim_is_rejected() {
        // Arrange
        let payload = format!(
            r#"{{"sub":"u","groups":["a","b"],"iss":"{ISSUER}","aud":"{AUDIENCE}","exp":2000}}"#
        );
        let token = encode_jwt(&valid_header(), &payload, SECRET);
        let config = AuthConfig {
            user_id_claim: "groups".to_string(),
            ..test_config()
        };

        // Act
        let result = validate_token(&token, &config, 1000);

        // Assert
        assert!(matches!(result, Err(AuthError::MissingUserId)));
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
            AuthError::MissingExpiry,
            AuthError::MissingUserId,
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

    // -- Secret length --

    #[test]
    fn secret_below_minimum_length_is_rejected() {
        // Arrange
        let secret = vec![0u8; MIN_SECRET_BYTES - 1];

        // Act
        let result = validate_secret(&secret);

        // Assert
        assert!(matches!(result, Err(AuthError::Config(_))));
    }

    #[test]
    fn empty_secret_is_rejected() {
        // Arrange
        let secret: Vec<u8> = Vec::new();

        // Act
        let result = validate_secret(&secret);

        // Assert
        assert!(matches!(result, Err(AuthError::Config(_))));
    }

    #[test]
    fn secret_at_minimum_length_is_accepted() {
        // Arrange
        let secret = vec![0u8; MIN_SECRET_BYTES];

        // Act
        let result = validate_secret(&secret);

        // Assert
        assert!(result.is_ok());
    }

    // -- Host clock --

    #[test]
    fn zero_clock_is_rejected() {
        // Arrange
        let token = encode_jwt(&valid_header(), &valid_payload(2000), SECRET);
        let config = test_config();

        // Act
        let result = validate_token(&token, &config, 0);

        // Assert
        assert!(matches!(result, Err(AuthError::Config(_))));
    }

    // -- Log message escaping --

    #[test]
    fn unsupported_algorithm_log_message_escapes_control_chars() {
        // Arrange
        let err = AuthError::UnsupportedAlgorithm("HS256\ninjected log line".to_string());

        // Act
        let msg = err.log_message();

        // Assert
        assert!(!msg.contains('\n'));
        assert!(msg.contains("\\n"));
    }

    // -- Token type (typ) --

    #[test]
    fn typ_not_checked_when_token_type_unset() {
        // Arrange
        let header = r#"{"alg":"HS256","typ":"unexpected-type"}"#;
        let token = encode_jwt(header, &valid_payload(2000), SECRET);
        let config = test_config();

        // Act
        let result = validate_token(&token, &config, 1000);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn matching_token_type_is_accepted() {
        // Arrange
        let header = r#"{"alg":"HS256","typ":"at+jwt"}"#;
        let token = encode_jwt(header, &valid_payload(2000), SECRET);
        let config = AuthConfig {
            token_type: Some("at+jwt".to_string()),
            ..test_config()
        };

        // Act
        let result = validate_token(&token, &config, 1000);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn token_type_match_is_case_insensitive_and_ignores_media_prefix() {
        // Arrange
        let header = r#"{"alg":"HS256","typ":"application/AT+JWT"}"#;
        let token = encode_jwt(header, &valid_payload(2000), SECRET);
        let config = AuthConfig {
            token_type: Some("at+jwt".to_string()),
            ..test_config()
        };

        // Act
        let result = validate_token(&token, &config, 1000);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn mismatched_token_type_is_rejected() {
        // Arrange
        let header = r#"{"alg":"HS256","typ":"JWT"}"#;
        let token = encode_jwt(header, &valid_payload(2000), SECRET);
        let config = AuthConfig {
            token_type: Some("at+jwt".to_string()),
            ..test_config()
        };

        // Act
        let result = validate_token(&token, &config, 1000);

        // Assert
        assert!(matches!(result, Err(AuthError::TypeMismatch)));
    }

    #[test]
    fn missing_typ_is_rejected_when_token_type_configured() {
        // Arrange
        let header = r#"{"alg":"HS256"}"#;
        let token = encode_jwt(header, &valid_payload(2000), SECRET);
        let config = AuthConfig {
            token_type: Some("JWT".to_string()),
            ..test_config()
        };

        // Act
        let result = validate_token(&token, &config, 1000);

        // Assert
        assert!(matches!(result, Err(AuthError::TypeMismatch)));
    }

    // -- Clock skew leeway --

    #[test]
    fn leeway_accepts_recently_expired_token() {
        // Arrange
        let token = encode_jwt(&valid_header(), &valid_payload(1000), SECRET);
        let config = AuthConfig {
            clock_skew_leeway_seconds: 60,
            ..test_config()
        };

        // Act
        let result = validate_token(&token, &config, 1030);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn leeway_rejects_token_expired_beyond_window() {
        // Arrange
        let token = encode_jwt(&valid_header(), &valid_payload(1000), SECRET);
        let config = AuthConfig {
            clock_skew_leeway_seconds: 60,
            ..test_config()
        };

        // Act
        let result = validate_token(&token, &config, 1061);

        // Assert
        assert!(matches!(result, Err(AuthError::TokenExpired)));
    }

    #[test]
    fn leeway_accepts_not_yet_valid_token_within_window() {
        // Arrange
        let payload = format!(
            r#"{{"sub":"user-1","iss":"{ISSUER}","aud":"{AUDIENCE}","exp":3000,"nbf":2000}}"#
        );
        let token = encode_jwt(&valid_header(), &payload, SECRET);
        let config = AuthConfig {
            clock_skew_leeway_seconds: 60,
            ..test_config()
        };

        // Act
        let result = validate_token(&token, &config, 1950);

        // Assert
        assert!(result.is_ok());
    }

    // -- Leeway parsing --

    #[test]
    fn parse_leeway_absent_defaults_to_zero() {
        // Arrange
        let raw = None;

        // Act
        let result = parse_leeway_seconds(raw);

        // Assert
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn parse_leeway_valid_number_is_parsed() {
        // Arrange
        let raw = Some("60");

        // Act
        let result = parse_leeway_seconds(raw);

        // Assert
        assert_eq!(result.unwrap(), 60);
    }

    #[test]
    fn parse_leeway_non_numeric_is_rejected() {
        // Arrange
        let raw = Some("soon");

        // Act
        let result = parse_leeway_seconds(raw);

        // Assert
        assert!(matches!(result, Err(AuthError::Config(_))));
    }

    // -- Revocation (jti) --

    #[test]
    fn revoked_token_is_rejected() {
        // Arrange
        let payload = format!(
            r#"{{"sub":"user-1","iss":"{ISSUER}","aud":"{AUDIENCE}","exp":2000,"jti":"revoked-1"}}"#
        );
        let token = encode_jwt(&valid_header(), &payload, SECRET);
        let config = AuthConfig {
            revoked_jti: HashSet::from(["revoked-1".to_string()]),
            ..test_config()
        };

        // Act
        let result = validate_token(&token, &config, 1000);

        // Assert
        assert!(matches!(result, Err(AuthError::TokenRevoked)));
    }

    #[test]
    fn non_revoked_token_is_accepted() {
        // Arrange
        let payload = format!(
            r#"{{"sub":"user-1","iss":"{ISSUER}","aud":"{AUDIENCE}","exp":2000,"jti":"allowed-1"}}"#
        );
        let token = encode_jwt(&valid_header(), &payload, SECRET);
        let config = AuthConfig {
            revoked_jti: HashSet::from(["revoked-1".to_string()]),
            ..test_config()
        };

        // Act
        let result = validate_token(&token, &config, 1000);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn missing_jti_rejected_when_revocation_enabled() {
        // Arrange
        let token = encode_jwt(&valid_header(), &valid_payload(2000), SECRET);
        let config = AuthConfig {
            revoked_jti: HashSet::from(["revoked-1".to_string()]),
            ..test_config()
        };

        // Act
        let result = validate_token(&token, &config, 1000);

        // Assert
        assert!(matches!(result, Err(AuthError::MissingJti)));
    }

    #[test]
    fn jti_not_required_when_revocation_disabled() {
        // Arrange
        let token = encode_jwt(&valid_header(), &valid_payload(2000), SECRET);
        let config = test_config();

        // Act
        let result = validate_token(&token, &config, 1000);

        // Assert
        assert!(result.is_ok());
    }

    // -- Claim value validation --

    #[test]
    fn user_id_with_control_char_is_rejected() {
        // Arrange
        let payload =
            format!(r#"{{"sub":"user\n42","iss":"{ISSUER}","aud":"{AUDIENCE}","exp":2000}}"#);
        let token = encode_jwt(&valid_header(), &payload, SECRET);
        let config = test_config();

        // Act
        let result = validate_token(&token, &config, 1000);

        // Assert
        assert!(matches!(result, Err(AuthError::InvalidClaimValue)));
    }

    #[test]
    fn empty_user_id_is_rejected() {
        // Arrange
        let payload = format!(r#"{{"sub":"","iss":"{ISSUER}","aud":"{AUDIENCE}","exp":2000}}"#);
        let token = encode_jwt(&valid_header(), &payload, SECRET);
        let config = test_config();

        // Act
        let result = validate_token(&token, &config, 1000);

        // Assert
        assert!(matches!(result, Err(AuthError::InvalidClaimValue)));
    }

    #[test]
    fn tenant_id_with_control_char_is_rejected() {
        // Arrange
        let payload = format!(
            r#"{{"sub":"user-1","tenant_id":"acme\r\nx","iss":"{ISSUER}","aud":"{AUDIENCE}","exp":2000}}"#
        );
        let token = encode_jwt(&valid_header(), &payload, SECRET);
        let config = AuthConfig {
            tenant_id_claim: Some("tenant_id".to_string()),
            ..test_config()
        };

        // Act
        let result = validate_token(&token, &config, 1000);

        // Assert
        assert!(matches!(result, Err(AuthError::InvalidClaimValue)));
    }

    #[test]
    fn is_safe_header_value_accepts_normal_and_rejects_control_and_empty() {
        // Arrange & Act
        let normal = is_safe_header_value("user-42");
        let empty = is_safe_header_value("");
        let newline = is_safe_header_value("a\nb");

        // Assert
        assert!(normal);
        assert!(!empty);
        assert!(!newline);
    }
}
