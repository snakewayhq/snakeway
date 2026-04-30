use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use subtle::ConstantTimeEq;

use super::SecretToken;

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
            matched |= token
                .expose_digest()
                .ct_eq(presented_hash.as_slice())
                .unwrap_u8();
        }
        matched == 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::BearerAuthSpec;
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
}
