use crate::types::{AdminAuthSpec, BearerAuthSpec, Origin};
use crate::validation::{
    MIN_TOKEN_LENGTH, TokenFileIssue, ValidateSpec, ValidationReportDeprecated, parse_token_file,
};

impl ValidateSpec for AdminAuthSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReportDeprecated) {
        if let Some(bearer) = &self.bearer {
            bearer.validate(origin, report);
        } else {
            report.admin_auth_missing(origin);
        }
    }
}

impl ValidateSpec for BearerAuthSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReportDeprecated) {
        let path = self.token_file.as_path();

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
}
