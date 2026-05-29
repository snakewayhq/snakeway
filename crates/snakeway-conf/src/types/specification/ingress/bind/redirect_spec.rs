use crate::types::HclInt;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Default, Serialize, Clone)]
pub struct RedirectSpec {
    pub port: HclInt,
    pub status: HclInt,
}
