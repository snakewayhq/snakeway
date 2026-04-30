use crate::types::BearerAuthSpec;
use crate::validation::validator::{TokenFileIssue, parse_token_file};
use std::fmt;
use std::path::PathBuf;

use super::{BearerAuthConfig, SecretToken};

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

    fn write_tokens(contents: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().expect("tempfile");
        f.write_all(contents.as_bytes()).expect("write");
        f
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
