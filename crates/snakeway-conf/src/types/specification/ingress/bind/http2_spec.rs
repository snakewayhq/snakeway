use crate::types::HclInt;
use serde::{Deserialize, Serialize};

/// HTTP/2 server tuning for a bind. All fields are optional; unset fields
/// keep Pingora's bounded defaults.
#[derive(Debug, Deserialize, Default, Serialize, Clone)]
pub struct Http2Spec {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_concurrent_streams: Option<HclInt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_header_list_size: Option<HclInt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_window_size: Option<HclInt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_connection_window_size: Option<HclInt>,
}
