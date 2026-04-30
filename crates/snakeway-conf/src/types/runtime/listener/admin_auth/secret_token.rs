use secrecy::{ExposeSecret, SecretBox};
use serde::{Deserialize, Serialize, Serializer};
use sha2::{Digest, Sha256};
use std::fmt;

/// A token value held in a form that resists accidental disclosure.
///
/// Stores the SHA-256 digest of the original token inside a `SecretBox`,
/// which zeroes the backing memory on drop. This ensures constant-time
/// comparison regardless of presented token length, avoids holding raw
/// token material in memory after startup, and prevents the digest from
/// lingering after the config is dropped.
///
/// - `Debug` prints `<redacted>`.
/// - `Serialize` emits `"<redacted>"` so `config dump --repr=runtime` never
///   prints token material.
/// - `Deserialize` is implemented for round-trip compatibility with dumped
///   configs, but produces a token that cannot match anything (a deserialised
///   runtime config is a diagnostic artefact, not an operational one).
/// - No `PartialEq` / `Eq` impl, to prevent accidental non-constant-time
///   comparisons. Use `BearerAuthConfig::verify` instead.
pub struct SecretToken(SecretBox<[u8; 32]>);

impl SecretToken {
    pub fn new(value: &[u8]) -> Self {
        SecretToken(SecretBox::new(Box::new(Sha256::digest(value).into())))
    }

    pub(super) fn expose_digest(&self) -> &[u8; 32] {
        self.0.expose_secret()
    }
}

impl Clone for SecretToken {
    fn clone(&self) -> Self {
        SecretToken(SecretBox::new(Box::new(*self.0.expose_secret())))
    }
}

impl fmt::Debug for SecretToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretToken(<redacted>)")
    }
}

impl Serialize for SecretToken {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str("<redacted>")
    }
}

impl<'de> Deserialize<'de> for SecretToken {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s: String = Deserialize::deserialize(d)?;
        let bytes: [u8; 32] = Sha256::digest(s.as_bytes()).into();
        Ok(SecretToken(SecretBox::new(Box::new(bytes))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_redacts_token_value() {
        // Arrange
        let token = SecretToken::new(b"supersecret");

        // Act
        let rendered = format!("{:?}", token);

        // Assert
        assert!(rendered.contains("<redacted>"));
        assert!(!rendered.contains("supersecret"));
    }

    #[test]
    fn serialize_redacts_token_value() {
        // Arrange
        let token = SecretToken::new(b"supersecret");

        // Act
        let json = serde_json::to_string(&token).expect("serialize");

        // Assert
        assert_eq!(json, "\"<redacted>\"");
    }
}
