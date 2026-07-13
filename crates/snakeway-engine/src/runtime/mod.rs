pub(crate) mod diff;
pub(crate) mod dns_refresh;
mod error;
mod state;
mod types;

#[cfg(test)]
pub(crate) use types::ResolvedAddr;
#[cfg(test)]
pub(crate) use types::UpstreamTcpRuntime;

pub(crate) use error::ReloadError;
pub(crate) use state::reload_runtime_state;
pub(crate) use types::{RuntimeState, ServiceRuntime, UpstreamId, UpstreamRuntime};

pub use state::build_runtime_state;
