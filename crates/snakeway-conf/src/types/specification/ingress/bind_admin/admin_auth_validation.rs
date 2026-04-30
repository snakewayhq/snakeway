use crate::types::{AdminAuthSpec, BearerAuthSpec, Origin};
use crate::validation::{
    MIN_TOKEN_LENGTH, TokenFileIssue, ValidateSpec, ValidationReport, parse_token_file,
};

impl ValidateSpec for AdminAuthSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        validate_admin_auth(self, origin, report);
    }
}

fn validate_admin_auth(auth: &AdminAuthSpec, origin: &Origin, report: &mut ValidationReport) {
    let Some(bearer) = &auth.bearer else {
        // No scheme is populated at all. When a second scheme is added here
        // (e.g., mTLS), check it before falling through to admin_auth_missing.
        report.admin_auth_missing(origin);
        return;
    };

    validate_bearer_auth(bearer, origin, report);
}

fn validate_bearer_auth(bearer: &BearerAuthSpec, origin: &Origin, report: &mut ValidationReport) {
    let path = bearer.token_file.as_path();

    // token_file must be set (reject the default empty path directly).
    if path.as_os_str().is_empty() {
        report.admin_auth_bearer_token_file_io_error(path, "token_file path is empty", origin);
        return;
    }

    // Parse the file and surface every issue.
    let outcome = parse_token_file(path);

    for err in &outcome.errors {
        match err {
            TokenFileIssue::FileIoError(msg) => {
                report.admin_auth_bearer_token_file_io_error(path, msg, origin);
            }
            TokenFileIssue::EmptyFile => {
                report.admin_auth_bearer_token_file_empty(path, origin);
            }
            TokenFileIssue::EmptyLine(line) => {
                report.admin_auth_bearer_empty_line(path, *line, origin);
            }
            TokenFileIssue::CommentNotAllowed(line) => {
                report.admin_auth_bearer_comment_line(path, *line, origin);
            }
            TokenFileIssue::TokenTooShort { line, len } => {
                report.admin_auth_bearer_token_too_short(
                    path,
                    *line,
                    *len,
                    MIN_TOKEN_LENGTH,
                    origin,
                );
            }
            TokenFileIssue::DuplicateToken { .. } => {
                // Duplicates are warnings and are enumerated below.
            }
        }
    }

    for warn in &outcome.warnings {
        if let TokenFileIssue::DuplicateToken {
            line,
            first_seen_line,
        } = warn
        {
            report.admin_auth_bearer_duplicate_token(path, *line, *first_seen_line, origin);
        }
    }
}
