use serde::Deserialize;

pub(crate) struct ValidatedToken {
    pub(crate) claims: JwtClaims,
}

#[derive(Deserialize)]
pub(crate) struct JwtHeader {
    pub(crate) alg: String,
    #[allow(dead_code)]
    typ: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct JwtClaims {
    #[serde(default)]
    pub(crate) iss: Option<String>,

    #[serde(default)]
    pub(crate) aud: Option<Audience>,

    #[serde(default)]
    sub: Option<String>,

    #[serde(default)]
    pub(crate) exp: Option<u64>,

    #[serde(default)]
    pub(crate) nbf: Option<u64>,

    #[serde(flatten)]
    extra: serde_json::Map<String, serde_json::Value>,
}

impl JwtClaims {
    pub(crate) fn get_claim(&self, name: &str) -> Option<String> {
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

#[derive(Deserialize)]
#[serde(untagged)]
pub(crate) enum Audience {
    Single(String),
    Multiple(Vec<String>),
}

impl Audience {
    pub(crate) fn contains(&self, expected: &str) -> bool {
        match self {
            Audience::Single(s) => s == expected,
            Audience::Multiple(v) => v.iter().any(|s| s == expected),
        }
    }
}

pub(crate) struct AuthConfig {
    pub(crate) secret: Vec<u8>,
    pub(crate) issuer: String,
    pub(crate) audience: String,
    pub(crate) user_id_claim: String,
    pub(crate) tenant_id_claim: Option<String>,
    pub(crate) public_paths: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- Audience --

    #[test]
    fn audience_single_matches() {
        // Arrange
        let aud = Audience::Single("https://api.example.com".to_string());

        // Act
        let matches = aud.contains("https://api.example.com");
        let no_match = aud.contains("https://other.example.com");

        // Assert
        assert!(matches);
        assert!(!no_match);
    }

    #[test]
    fn audience_multiple_matches_any() {
        // Arrange
        let aud = Audience::Multiple(vec![
            "https://api.example.com".to_string(),
            "https://admin.example.com".to_string(),
        ]);

        // Act
        let matches_first = aud.contains("https://api.example.com");
        let matches_second = aud.contains("https://admin.example.com");
        let no_match = aud.contains("https://other.example.com");

        // Assert
        assert!(matches_first);
        assert!(matches_second);
        assert!(!no_match);
    }

    // -- JwtClaims::get_claim --

    #[test]
    fn get_claim_sub() {
        // Arrange
        let claims: JwtClaims = serde_json::from_str(r#"{"sub": "user-42"}"#).unwrap();

        // Act
        let result = claims.get_claim("sub");

        // Assert
        assert_eq!(result, Some("user-42".to_string()));
    }

    #[test]
    fn get_claim_iss() {
        // Arrange
        let claims: JwtClaims =
            serde_json::from_str(r#"{"iss": "https://auth.example.com"}"#).unwrap();

        // Act
        let result = claims.get_claim("iss");

        // Assert
        assert_eq!(result, Some("https://auth.example.com".to_string()));
    }

    #[test]
    fn get_claim_extra_string() {
        // Arrange
        let claims: JwtClaims = serde_json::from_str(r#"{"tenant_id": "acme-corp"}"#).unwrap();

        // Act
        let result = claims.get_claim("tenant_id");

        // Assert
        assert_eq!(result, Some("acme-corp".to_string()));
    }

    #[test]
    fn get_claim_extra_number() {
        // Arrange
        let claims: JwtClaims = serde_json::from_str(r#"{"org_id": 99}"#).unwrap();

        // Act
        let result = claims.get_claim("org_id");

        // Assert
        assert_eq!(result, Some("99".to_string()));
    }

    #[test]
    fn get_claim_extra_bool() {
        // Arrange
        let claims: JwtClaims = serde_json::from_str(r#"{"admin": true}"#).unwrap();

        // Act
        let result = claims.get_claim("admin");

        // Assert
        assert_eq!(result, Some("true".to_string()));
    }

    #[test]
    fn get_claim_missing_returns_none() {
        // Arrange
        let claims: JwtClaims = serde_json::from_str(r#"{}"#).unwrap();

        // Act
        let sub = claims.get_claim("sub");
        let nonexistent = claims.get_claim("nonexistent");

        // Assert
        assert_eq!(sub, None);
        assert_eq!(nonexistent, None);
    }

    #[test]
    fn get_claim_array_value_returns_none() {
        // Arrange
        let claims: JwtClaims = serde_json::from_str(r#"{"roles": ["admin", "user"]}"#).unwrap();

        // Act
        let result = claims.get_claim("roles");

        // Assert
        assert_eq!(result, None);
    }
}
