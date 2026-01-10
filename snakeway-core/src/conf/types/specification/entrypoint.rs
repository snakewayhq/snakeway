use crate::conf::types::ServerSpec;
use serde::Deserialize;

/// Represents the top-level configuration file.
#[derive(Debug, Deserialize)]
pub struct EntrypointSpec {
    pub server: ServerSpec,
    pub include: IncludeSpec,
}

/// Represents the include section of the top-level config file.
/// The members are directory paths where sub-configuration files are located.
#[derive(Debug, Deserialize)]
pub struct IncludeSpec {
    pub devices: String,
    pub ingress: String,
}
