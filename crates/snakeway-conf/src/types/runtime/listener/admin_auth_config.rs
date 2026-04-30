use crate::types::BearerAuthSpec;
use crate::validation::validator::{TokenFileIssue, parse_token_file};
use serde::{Deserialize, Serialize, Serializer};
use sha2::{Digest, Sha256};
use std::fmt;
use std::path::PathBuf;
use subtle::ConstantTimeEq;

/// Admin listener authentication. At least one scheme must be populated;
/// this is enforced by validation.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AdminAuthConfig {
    #[serde(default)]
    pub bearer: Option<BearerAuthConfig>,
}

/// Resolved bearer-token authentication config.
///
/// `token_file` is retained for diagnostic output; the tokens themselves are
/// kept in `SecretToken` so they cannot accidentally be logged or printed.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BearerAuthConfig {
    pub token_file: PathBuf,
    pub tokens: Vec<SecretToken>,
}

impl BearerAuthConfig {
    /// Verify a presented token against every active token in constant time.
    ///
    /// Both sides are hashed with SHA-256 before comparison so that
    /// `ct_eq` always operates on fixed-length (32-byte) digests. Without
    /// this, `subtle::ConstantTimeEq` for `[u8]` returns 0 immediately
    /// when lengths differ, leaking the stored token length via timing.
    ///
    /// The loop accumulates match results with a bitwise OR so total
    /// timing is independent of which token matched or whether any token
    /// matched.
    pub fn verify(&self, presented: &[u8]) -> bool {
        let presented_hash = Sha256::digest(presented);
        let mut matched: u8 = 0;
        for token in &self.tokens {
            matched |= token.0.ct_eq(presented_hash.as_slice()).unwrap_u8();
        }
        matched == 1
    }
}

/// A token value held in a form that resists accidental disclosure.
///
/// Stores the SHA-256 digest of the original token, not the token itself.
/// This ensures constant-time comparison regardless of presented token
/// length and avoids holding raw token material in memory after startup.
///
/// - `Debug` prints `<redacted>`.
/// - `Serialize` emits `"<redacted>"` so `config dump --repr=runtime` never
///   prints token material.
/// - `Deserialize` is implemented for round-trip compatibility with dumped
///   configs, but produces a token that cannot match anything (a deserialised
///   runtime config is a diagnostic artefact, not an operational one).
/// - No `PartialEq` / `Eq` impl, to prevent accidental non-constant-time
///   comparisons. Use `BearerAuthConfig::verify` instead.
#[derive(Clone)]
pub struct SecretToken(Box<[u8]>);

impl SecretToken {
    pub fn new(value: &[u8]) -> Self {
        SecretToken(Sha256::digest(value).to_vec().into_boxed_slice())
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
        Ok(SecretToken(s.into_bytes().into_boxed_slice()))
    }
}

/// Error produced when lowering `BearerAuthSpec` to `BearerAuthConfig` fails.
///
/// Lowering should not fail in normal operation because validation confirms
/// the token file parses cleanly. If this error surfaces, it means the file
/// was mutated between validation and lowering, or validation was skipped.
#[derive(Debug)]
pub struct BearerAuthLowerError {
    pub token_file: PathBuf,
    pub(crate) issues: Vec<TokenFileIssue>,
}

impl fmt::Display for BearerAuthLowerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "failed to load bearer token file {}: {} issue(s)",
            self.token_file.display(),
            self.issues.len()
        )
    }
}

impl std::error::Error for BearerAuthLowerError {}

impl TryFrom<BearerAuthSpec> for BearerAuthConfig {
    type Error = BearerAuthLowerError;

    fn try_from(spec: BearerAuthSpec) -> Result<Self, Self::Error> {
        let outcome = parse_token_file(&spec.token_file);
        if !outcome.is_ok() {
            return Err(BearerAuthLowerError {
                token_file: spec.token_file,
                issues: outcome.errors,
            });
        }

        let tokens = outcome
            .tokens
            .iter()
            .map(|s| SecretToken::new(s.as_bytes()))
            .collect();

        Ok(BearerAuthConfig {
            token_file: spec.token_file,
            tokens,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn valid_token() -> &'static str {
        "a9f1c38de4b67029c5d1e97f4a0ebac12d3b8ffc84e1d27a05f6cb9e83d21a04"
    }

    fn write_tokens(contents: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().expect("tempfile");
        f.write_all(contents.as_bytes()).expect("write");
        f
    }

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

    #[test]
    fn verify_matches_valid_token() {
        // Arrange
        let file = write_tokens(&format!("{}\n", valid_token()));
        let spec = BearerAuthSpec {
            token_file: file.path().to_path_buf(),
            origin: Default::default(),
        };
        let cfg = BearerAuthConfig::try_from(spec).expect("lower");

        // Act
        let ok = cfg.verify(valid_token().as_bytes());

        // Assert
        assert!(ok);
    }

    #[test]
    fn verify_rejects_unknown_token() {
        // Arrange
        let file = write_tokens(&format!("{}\n", valid_token()));
        let spec = BearerAuthSpec {
            token_file: file.path().to_path_buf(),
            origin: Default::default(),
        };
        let cfg = BearerAuthConfig::try_from(spec).expect("lower");

        // Act
        let ok = cfg.verify(b"nope-not-the-real-token");

        // Assert
        assert!(!ok);
    }

    #[test]
    fn verify_accepts_any_active_token() {
        // Arrange
        let t1 = valid_token();
        let t2 = "7b4e19a2c5f8d3046e9b71c8a52f9e1d4c07bfa6e93d1c24b87a90fed362014c";
        let file = write_tokens(&format!("{t1}\n{t2}\n"));
        let spec = BearerAuthSpec {
            token_file: file.path().to_path_buf(),
            origin: Default::default(),
        };
        let cfg = BearerAuthConfig::try_from(spec).expect("lower");

        // Act
        let first_ok = cfg.verify(t1.as_bytes());
        let second_ok = cfg.verify(t2.as_bytes());

        // Assert
        assert!(first_ok);
        assert!(second_ok);
    }

    #[test]
    fn try_from_propagates_parse_errors() {
        // Arrange
        let file = write_tokens("too-short\n");
        let spec = BearerAuthSpec {
            token_file: file.path().to_path_buf(),
            origin: Default::default(),
        };

        // Act
        let result = BearerAuthConfig::try_from(spec);

        // Assert
        assert!(result.is_err());
    }
}
