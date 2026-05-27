mod bearer_auth_config;
mod secret_token;

pub use bearer_auth_config::*;
pub use secret_token::*;

use serde::{Deserialize, Serialize};

/// Admin listener authentication. At least one scheme must be populated;
/// this is enforced by validation.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AdminAuthConfig {
    #[serde(default)]
    pub bearer: Option<BearerAuthConfig>,
}
