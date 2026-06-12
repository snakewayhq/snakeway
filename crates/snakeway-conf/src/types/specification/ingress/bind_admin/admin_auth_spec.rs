use crate::validation::{MIN_TOKEN_LENGTH, TokenFileIssue, parse_token_file};
use confval::provenance::{Located, Report, Span};
use serde::Serialize;
use std::path::PathBuf;

/// Authentication block for the admin listener.
///
/// A scheme slot (currently `bearer`) must be populated. Future schemes
/// (mTLS client cert, Basic, etc.) can be added as additional sibling fields
/// without restructuring.
#[derive(Debug, Default, Clone, Serialize, confval::Spec)]
pub struct AdminAuthSpec {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[confval(nested)]
    pub bearer: Option<Located<BearerAuthSpec>>,
}

/// Bearer token authentication configuration.
#[derive(Debug, Default, Clone, Serialize, confval::Spec)]
pub struct BearerAuthSpec {
    pub token_file: Located<PathBuf>,
}

pub(crate) fn report_admin_auth_missing(span: Span, report: &mut Report) {
    report
        .error("bind_admin.auth is required")
        .at(span)
        .help(
            "Add an auth block, e.g. auth = { bearer = { token_file = \
             \"/etc/snakeway/admin.tokens\" } }",
        )
        .emit();
}

pub(crate) fn validate_admin_auth(spec: &AdminAuthSpec, span: Span, report: &mut Report) {
    match &spec.bearer {
        Some(bearer) => validate_bearer_auth(&bearer.value, report),
        None => report_admin_auth_missing(span, report),
    }
}

fn validate_bearer_auth(spec: &BearerAuthSpec, report: &mut Report) {
    let path = spec.token_file.value.as_path();
    let span = spec.token_file.span;

    if path.as_os_str().is_empty() {
        report
            .error(format!(
                "bearer token_file could not be read ({}): token_file path is empty",
                path.display()
            ))
            .at(span)
            .emit();
        return;
    }

    let outcome = parse_token_file(path);

    for err in &outcome.errors {
        match err {
            TokenFileIssue::FileIoError(msg) => {
                report
                    .error(format!(
                        "bearer token_file could not be read ({}): {}",
                        path.display(),
                        msg
                    ))
                    .at(span)
                    .emit();
            }
            TokenFileIssue::EmptyFile => {
                report
                    .error(format!("bearer token_file is empty: {}", path.display()))
                    .at(span)
                    .help("Add at least one token line (one token per line).")
                    .emit();
            }
            TokenFileIssue::EmptyLine(line) => {
                report
                    .error(format!(
                        "bearer token_file {} has an empty line at line {}",
                        path.display(),
                        line
                    ))
                    .at(span)
                    .help("Remove the blank line. Lines must be either a token or the end of the file.")
                    .emit();
            }
            TokenFileIssue::CommentNotAllowed(line) => {
                report
                    .error(format!(
                        "bearer token_file {} has a comment at line {}; comments are not permitted",
                        path.display(),
                        line
                    ))
                    .at(span)
                    .help("Remove the comment line.")
                    .emit();
            }
            TokenFileIssue::TokenTooShort { line, len } => {
                report
                    .error(format!(
                        "bearer token_file {} has a token at line {} that is {} bytes; minimum is {}",
                        path.display(),
                        line,
                        len,
                        MIN_TOKEN_LENGTH
                    ))
                    .at(span)
                    .help(
                        "Generate a token with `openssl rand -hex 32` (or any source of at \
                         least 32 bytes of random data).",
                    )
                    .emit();
            }
            TokenFileIssue::DuplicateToken { .. } => {}
        }
    }

    for warn in &outcome.warnings {
        if let TokenFileIssue::DuplicateToken {
            line,
            first_seen_line,
        } = warn
        {
            report
                .warning(format!(
                    "bearer token_file {} has a duplicate token at line {} (first seen at line {})",
                    path.display(),
                    line,
                    first_seen_line
                ))
                .at(span)
                .help("Remove the duplicate entry.")
                .emit();
        }
    }
}
