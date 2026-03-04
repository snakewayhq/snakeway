use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IdentityField {
    Asn,
    Aso,
    Country,
    Region,
    ConnectionType,
    Bot,
    Device,
}
