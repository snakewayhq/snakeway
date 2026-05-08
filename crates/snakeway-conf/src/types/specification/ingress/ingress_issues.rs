use crate::types::HclOrigin;
use confval::ValidationIssue;

pub(crate) fn missing_bind(origin: &HclOrigin) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error(
        "ingress config must have a bind or bind_admin declaration",
        origin.clone(),
    )
}
