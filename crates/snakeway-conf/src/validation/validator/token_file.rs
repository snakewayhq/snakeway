use std::fs;
use std::path::Path;

/// Minimum byte length for a bearer token, enforced after trimming whitespace.
///
/// Chosen so that an operator using `openssl rand -hex 16` (32 hex chars, 128
/// bits of entropy) passes, and obvious footguns like a human-chosen
/// password do not.
pub(crate) const MIN_TOKEN_LENGTH: usize = 32;

/// A single issue discovered while parsing a token file. One-based line
/// numbers are used so error messages match what operators see in an editor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TokenFileIssue {
    FileIoError(String),
    EmptyFile,
    EmptyLine(usize),
    CommentNotAllowed(usize),
    TokenTooShort { line: usize, len: usize },
    DuplicateToken { line: usize, first_seen_line: usize },
}

/// Outcome of parsing a token file. On success the caller receives the token
/// list plus any non-fatal warnings (e.g., duplicates). On failure, every
/// issue discovered in one pass is returned so the operator can fix them all
/// at once.
#[derive(Debug, Clone)]
pub(crate) struct TokenFileOutcome {
    pub(crate) tokens: Vec<String>,
    pub(crate) errors: Vec<TokenFileIssue>,
    pub(crate) warnings: Vec<TokenFileIssue>,
}

impl TokenFileOutcome {
    pub(crate) fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Parse a bearer token file.
///
/// Format:
/// - One token per line.
/// - Leading and trailing whitespace is trimmed per line.
/// - Empty lines are errors (not silently skipped) to avoid hiding format
///   mistakes.
/// - Comments (lines starting with `#`) are rejected; this keeps the format
///   unambiguous and machine-editable.
/// - Every token must be at least `MIN_TOKEN_LENGTH` bytes.
/// - Duplicate tokens are warnings, not errors.
pub(crate) fn parse_token_file(path: &Path) -> TokenFileOutcome {
    let contents = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            return TokenFileOutcome {
                tokens: Vec::new(),
                errors: vec![TokenFileIssue::FileIoError(e.to_string())],
                warnings: Vec::new(),
            };
        }
    };

    let mut tokens: Vec<String> = Vec::new();
    let mut errors: Vec<TokenFileIssue> = Vec::new();
    let mut warnings: Vec<TokenFileIssue> = Vec::new();
    let mut first_seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    let mut any_line_present = false;
    for (idx, raw) in contents.lines().enumerate() {
        let line_no = idx + 1;
        any_line_present = true;
        let trimmed = raw.trim();

        if trimmed.is_empty() {
            errors.push(TokenFileIssue::EmptyLine(line_no));
            continue;
        }

        if trimmed.starts_with('#') {
            errors.push(TokenFileIssue::CommentNotAllowed(line_no));
            continue;
        }

        if trimmed.len() < MIN_TOKEN_LENGTH {
            errors.push(TokenFileIssue::TokenTooShort {
                line: line_no,
                len: trimmed.len(),
            });
            continue;
        }

        let token = trimmed.to_string();
        if let Some(&first_seen_line) = first_seen.get(&token) {
            warnings.push(TokenFileIssue::DuplicateToken {
                line: line_no,
                first_seen_line,
            });
        } else {
            first_seen.insert(token.clone(), line_no);
        }
        tokens.push(token);
    }

    if (!any_line_present || tokens.is_empty()) && errors.is_empty() {
        errors.push(TokenFileIssue::EmptyFile);
    }

    TokenFileOutcome {
        tokens,
        errors,
        warnings,
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
    fn parses_multiple_valid_tokens() {
        // Arrange
        let file = write_tokens(
            "a9f1c38de4b67029c5d1e97f4a0ebac12d3b8ffc84e1d27a05f6cb9e83d21a04\n\
             7b4e19a2c5f8d3046e9b71c8a52f9e1d4c07bfa6e93d1c24b87a90fed362014c\n",
        );

        // Act
        let out = parse_token_file(file.path());

        // Assert
        assert!(out.is_ok(), "expected ok, got errors: {:?}", out.errors);
        assert_eq!(out.tokens.len(), 2);
        assert!(out.warnings.is_empty());
    }

    #[test]
    fn missing_file_returns_io_error() {
        // Arrange
        let path = std::path::PathBuf::from("/nonexistent/token/file.dat");

        // Act
        let out = parse_token_file(&path);

        // Assert
        assert!(!out.is_ok());
        assert!(matches!(out.errors[0], TokenFileIssue::FileIoError(_)));
    }

    #[test]
    fn empty_file_is_error() {
        // Arrange
        let file = write_tokens("");

        // Act
        let out = parse_token_file(file.path());

        // Assert
        assert_eq!(out.errors, vec![TokenFileIssue::EmptyFile]);
    }

    #[test]
    fn whitespace_only_lines_are_errors() {
        // Arrange
        let file = write_tokens("   \n\t\n");

        // Act
        let out = parse_token_file(file.path());

        // Assert
        assert!(!out.is_ok());
        assert!(
            out.errors
                .iter()
                .any(|e| matches!(e, TokenFileIssue::EmptyLine(_)))
        );
    }

    #[test]
    fn comments_rejected() {
        // Arrange
        let file = write_tokens(
            "# a comment line\n\
             a9f1c38de4b67029c5d1e97f4a0ebac12d3b8ffc84e1d27a05f6cb9e83d21a04\n",
        );

        // Act
        let out = parse_token_file(file.path());

        // Assert
        assert!(
            out.errors
                .iter()
                .any(|e| matches!(e, TokenFileIssue::CommentNotAllowed(1)))
        );
    }

    #[test]
    fn token_too_short_rejected() {
        // Arrange
        let file = write_tokens("password123\n");

        // Act
        let out = parse_token_file(file.path());

        // Assert
        assert!(
            out.errors
                .iter()
                .any(|e| matches!(e, TokenFileIssue::TokenTooShort { .. }))
        );
    }

    #[test]
    fn duplicate_tokens_warning_not_error() {
        // Arrange
        let t = "a9f1c38de4b67029c5d1e97f4a0ebac12d3b8ffc84e1d27a05f6cb9e83d21a04";
        let file = write_tokens(&format!("{t}\n{t}\n"));

        // Act
        let out = parse_token_file(file.path());

        // Assert
        assert!(out.is_ok());
        assert_eq!(out.tokens.len(), 2);
        assert!(
            out.warnings
                .iter()
                .any(|w| matches!(w, TokenFileIssue::DuplicateToken { .. }))
        );
    }

    #[test]
    fn trims_whitespace_around_token() {
        // Arrange
        let t = "a9f1c38de4b67029c5d1e97f4a0ebac12d3b8ffc84e1d27a05f6cb9e83d21a04";
        let file = write_tokens(&format!("   {t}\t\n"));

        // Act
        let out = parse_token_file(file.path());

        // Assert
        assert!(out.is_ok());
        assert_eq!(out.tokens, vec![t.to_string()]);
    }
}
