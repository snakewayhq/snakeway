use crate::types::HclOrigin;
use confval::ValidationIssue;
use std::path::Path;

pub(crate) fn admin_auth_missing(origin: &HclOrigin) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error_with_help(
        "bind_admin.auth is required",
        origin.clone(),
        "Add an auth block, e.g. auth = { bearer = { token_file = \"/etc/snakeway/admin.tokens\" } }",
    )
}

pub(crate) fn admin_auth_bearer_token_file_io_error(
    path: &Path,
    message: &str,
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error(
        format!(
            "bearer token_file could not be read ({}): {}",
            path.display(),
            message
        ),
        origin.clone(),
    )
}

pub(crate) fn admin_auth_bearer_token_file_empty(
    path: &Path,
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error_with_help(
        format!("bearer token_file is empty: {}", path.display()),
        origin.clone(),
        "Add at least one token line (one token per line).",
    )
}

pub(crate) fn admin_auth_bearer_empty_line(
    path: &Path,
    line: usize,
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error_with_help(
        format!(
            "bearer token_file {} has an empty line at line {}",
            path.display(),
            line
        ),
        origin.clone(),
        "Remove the blank line. Lines must be either a token or the end of the file.",
    )
}

pub(crate) fn admin_auth_bearer_comment_line(
    path: &Path,
    line: usize,
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error_with_help(
        format!(
            "bearer token_file {} has a comment at line {}; comments are not permitted",
            path.display(),
            line
        ),
        origin.clone(),
        "Remove the comment line.",
    )
}

pub(crate) fn admin_auth_bearer_token_too_short(
    path: &Path,
    line: usize,
    len: usize,
    min: usize,
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error_with_help(
        format!(
            "bearer token_file {} has a token at line {} that is {} bytes; minimum is {}",
            path.display(),
            line,
            len,
            min
        ),
        origin.clone(),
        "Generate a token with `openssl rand -hex 32` (or any source of at least 32 bytes of random data).",
    )
}

pub(crate) fn admin_auth_bearer_duplicate_token(
    path: &Path,
    line: usize,
    first_seen_line: usize,
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::warning_with_help(
        format!(
            "bearer token_file {} has a duplicate token at line {} (first seen at line {})",
            path.display(),
            line,
            first_seen_line
        ),
        origin.clone(),
        "Remove the duplicate entry.",
    )
}
