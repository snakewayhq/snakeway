//! Test upstream servers.
//!
//! Every starter binds its listener before returning and reports the bound
//! port, so the port is live from the moment the caller sees it and cannot be
//! taken by another test or process between allocation and bind. The one
//! exception is [`start_http_upstream_on`], which exists for tests where the
//! proxy already carries a port and the upstream comes up later.

mod counting;
mod echo;
mod fault;
mod grpc;
mod h2;
mod plain;
mod tls;
mod ws;

pub use counting::CountingUpstream;
pub use echo::start_http_upstream_that_echoes;
pub use fault::DripBodyUpstream;
pub use fault::IncompleteHeadersUpstream;
pub use fault::KillIdleUpstream;
pub use fault::start_chunked_response_with_trailers_upstream;
pub use fault::start_conflicting_cl_te_upstream;
pub use fault::start_drip_body_upstream;
pub use fault::start_duplicate_content_length_upstream;
pub use fault::start_garbage_upstream;
pub use fault::start_hop_by_hop_response_upstream;
pub use fault::start_hundred_then_final_upstream;
pub use fault::start_incomplete_headers_upstream;
pub use fault::start_kill_idle_upstream;
pub use fault::start_reset_mid_response_upstream;
pub use fault::start_truncated_body_upstream;
pub use grpc::helloworld;
pub use grpc::start_grpc_upstream;
pub use h2::H2Upstream;
pub use h2::start_h2_tls_upstream;
pub use plain::start_http_upstream;
pub use plain::start_http_upstream_on;
pub use plain::start_http_upstream_that_echoes_headers;
pub use plain::start_http_upstream_that_echoes_request_line;
pub use plain::start_http_upstream_that_hangs;
pub use plain::start_http_upstream_that_reads_request;
pub use plain::start_http_upstream_that_returns_5xx;
pub use plain::start_http_upstream_with_large_response;
pub use plain::start_slow_http_upstream;
pub use plain::start_unix_http_upstream;
pub use tls::generate_upstream_identity;
pub use tls::start_https_upstream;
pub use tls::start_unix_https_upstream;
pub use ws::start_ws_upstream;
