mod bearer_auth_config;
mod secret_token;

pub use bearer_auth_config::*;
pub use secret_token::*;

use serde::{Deserialize, Serialize};

/// Admin listener authentication.
/// Validation requires at least one scheme to be populated.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AdminAuthConfig {
    #[serde(default)]
    pub bearer: Option<BearerAuthConfig>,
}
