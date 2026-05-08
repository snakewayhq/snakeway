use crate::types::HclOrigin;
use confval::ValidationIssue;
use std::path::Path;

pub(crate) fn invalid_static_dir(dir: &Path, origin: &HclOrigin) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error(
        format!("invalid static directory: {}", dir.display()),
        origin.clone(),
    )
}

pub(crate) fn invalid_static_dir_must_be_absolute(
    dir: &Path,
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error(
        format!(
            "static file directory must be an absolute path: {}",
            dir.display()
        ),
        origin.clone(),
    )
}
