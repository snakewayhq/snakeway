use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Default, Serialize, Clone)]
pub struct RedirectSpec {
    pub port: u16,
    pub status: u16,
}
