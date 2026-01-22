use crate::conf::types::Origin;
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RequestFilterDeviceSpec {
    #[serde(skip)]
    pub origin: Origin,

    /// Whether this request filter device is enabled.
    pub enable: bool,

    //-------------------------------------------------------------------------
    // Method policy
    //-------------------------------------------------------------------------
    #[serde(default)]
    pub allow_methods: Vec<String>,

    #[serde(default)]
    pub deny_methods: Vec<String>,

    //-------------------------------------------------------------------------
    // Header policy
    //-------------------------------------------------------------------------
    #[serde(default)]
    pub deny_headers: Vec<String>,

    #[serde(default)]
    pub allow_headers: Vec<String>,

    #[serde(default)]
    pub required_headers: Vec<String>,

    //-------------------------------------------------------------------------
    // Size limits
    //-------------------------------------------------------------------------
    #[serde(default = "default_max_header_bytes")]
    pub max_header_bytes: usize,
    #[serde(default = "default_max_body_bytes")]
    pub max_body_bytes: usize,

    //-------------------------------------------------------------------------
    // Override the default granular deny status with a device-scoped value.
    //-------------------------------------------------------------------------
    pub deny_status: Option<u16>,
}

fn default_max_header_bytes() -> usize {
    16 * 1024 // 16 KB
}

fn default_max_body_bytes() -> usize {
    1 * 1024 * 1024 // 1 MB
}
