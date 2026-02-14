use crate::conf::types::ServerSpec;
use serde::{Deserialize, Serialize};

/// Represents the top-level configuration file.
#[derive(Debug, Deserialize, Serialize, Default)]
pub struct EntrypointSpec {
    pub server: ServerSpec,
    pub include: IncludeSpec,
}

/// Represents the include section of the top-level config file.
/// The members are directory paths where sub-configuration files are located.
#[derive(Debug, Deserialize, Serialize)]
pub struct IncludeSpec {
    pub devices: String,
    pub ingresses: String,
}

impl Default for IncludeSpec {
    fn default() -> Self {
        Self {
            devices: "device.d/*.hcl".to_string(),
            ingresses: "ingress.d/*.hcl".to_string(),
        }
    }
}
