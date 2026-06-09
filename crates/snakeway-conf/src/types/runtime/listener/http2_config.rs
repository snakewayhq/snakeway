use crate::types::Http2Spec;
use serde::{Deserialize, Serialize};

/// Resolved HTTP/2 server tuning for a listener.
/// Unset fields keep Pingora's bounded defaults.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Http2Config {
    pub max_concurrent_streams: Option<u32>,
    pub max_header_list_size: Option<u32>,
    pub initial_window_size: Option<u32>,
    pub initial_connection_window_size: Option<u32>,
}

impl TryFrom<Http2Spec> for Http2Config {
    type Error = String;

    fn try_from(spec: Http2Spec) -> Result<Self, Self::Error> {
        Ok(Self {
            max_concurrent_streams: to_u32(spec.max_concurrent_streams, "max_concurrent_streams")?,
            max_header_list_size: to_u32(spec.max_header_list_size, "max_header_list_size")?,
            initial_window_size: to_u32(spec.initial_window_size, "initial_window_size")?,
            initial_connection_window_size: to_u32(
                spec.initial_connection_window_size,
                "initial_connection_window_size",
            )?,
        })
    }
}

/// Validation guarantees the range; an error here means validation was
/// skipped, so surface it as a lowering error rather than truncating.
fn to_u32(value: Option<i64>, field: &str) -> Result<Option<u32>, String> {
    value
        .map(|v| {
            u32::try_from(v).map_err(|_| {
                format!(
                    "http2.{field} is out of range (bug: validation should have caught this): {v}"
                )
            })
        })
        .transpose()
}
