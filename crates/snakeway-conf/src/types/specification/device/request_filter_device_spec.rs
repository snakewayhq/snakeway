use crate::types::{HclInt, HclOrigin};
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RequestFilterDeviceSpec {
    #[serde(skip)]
    pub origin: HclOrigin,

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
    pub max_header_bytes: HclInt,
    #[serde(default = "default_max_body_bytes")]
    pub max_body_bytes: HclInt,
    #[serde(default = "default_max_suspicious_body_bytes")]
    pub max_suspicious_body_bytes: HclInt,

    //-------------------------------------------------------------------------
    // Timeouts
    //-------------------------------------------------------------------------
    /// Maximum time (in seconds) to wait for each chunk of request body data
    /// from the client.  If the client stalls mid-body for longer than this
    /// duration, the connection is terminated.  This prevents slowloris-style
    /// attacks that hold upstream resources by trickling body bytes.
    ///
    /// Applied to the downstream read timeout via Pingora's session API.
    /// `None` keeps Pingora's default (60 s).
    #[serde(default)]
    pub client_body_timeout_seconds: Option<HclInt>,

    //-------------------------------------------------------------------------
    // Override the default granular deny status with a device-scoped value.
    //-------------------------------------------------------------------------
    pub deny_status: Option<HclInt>,

    //-------------------------------------------------------------------------
    // Path scoping
    //-------------------------------------------------------------------------
    /// Optional path prefixes this device applies to. Empty means all paths.
    #[serde(default)]
    pub paths: Vec<String>,
}

fn default_max_header_bytes() -> HclInt {
    16 * 1024 // 16 KB
}

fn default_max_body_bytes() -> HclInt {
    1024 * 1024 // 1 MB
}
fn default_max_suspicious_body_bytes() -> HclInt {
    8 * 1024 // 8 KB
}
