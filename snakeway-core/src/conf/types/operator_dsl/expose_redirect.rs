use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Default, Serialize)]
pub struct ExposeRedirectConfig {
    pub addr: String,
    pub to: String,
    pub status: u16,
}
