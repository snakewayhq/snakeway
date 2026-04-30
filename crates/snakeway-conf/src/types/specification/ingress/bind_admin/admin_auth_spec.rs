use crate::types::Origin;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Authentication block for the admin listener.
///
/// A scheme slot (currently `bearer`) must be populated. Future schemes
/// (mTLS client cert, Basic, etc.) can be added as additional sibling fields
/// without restructuring.
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct AdminAuthSpec {
    #[serde(skip)]
    pub origin: Origin,
    pub bearer: Option<BearerAuthSpec>,
}

/// Bearer token authentication configuration.
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct BearerAuthSpec {
    #[serde(skip)]
    pub origin: Origin,
    pub token_file: PathBuf,
}
