use crate::types::Http2Spec;
use confval::prelude::narrow;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, confval::Config)]
#[confval(lower_from = Http2Spec)]
pub struct Http2Config {
    #[confval(lower(from = max_concurrent_streams, with = narrow::i64_to_u32))]
    pub max_concurrent_streams: Option<u32>,
    #[confval(lower(from = max_header_list_size, with = narrow::i64_to_u32))]
    pub max_header_list_size: Option<u32>,
    #[confval(lower(from = initial_window_size, with = narrow::i64_to_u32))]
    pub initial_window_size: Option<u32>,
    #[confval(lower(from = initial_connection_window_size, with = narrow::i64_to_u32))]
    pub initial_connection_window_size: Option<u32>,
}
