use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct TlsSpec {
    pub cert: String,
    pub key: String,
}
