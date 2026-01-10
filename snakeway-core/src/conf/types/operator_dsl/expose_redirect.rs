use crate::conf::types::Origin;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Default, Serialize)]
pub struct ExposeRedirectConfig {
    #[serde(skip)]
    pub origin: Origin,
    pub addr: String,
    pub to: String,
    pub status: u16,
}
