use serde::{Deserialize, Serialize};

/// Paths are validated and resolved during config validation.
/// Runtime code assumes these values are valid.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct TlsConfig {
    pub cert: String,
    pub key: String,
}
