use snakeway_proxy::ReloadHandle;
use std::sync::Arc;

/// Handle to a server running in a background thread.
///
/// Holds the control-plane Tokio runtime so that spawned tasks (reload
/// loop, cert reconciliation) stay alive for the lifetime of the server.
pub struct RuntimeServer {
    pub reload: Arc<ReloadHandle>,
    /// Kept alive to allow spawned tasks (reload loop, cert reconciliation) to
    /// live long.
    /// Never read directly.
    /// Drop is the only consumer.
    _control_rt: tokio::runtime::Runtime,
}

impl RuntimeServer {
    pub fn new(reload: Arc<ReloadHandle>, control_rt: tokio::runtime::Runtime) -> Self {
        Self {
            reload,
            _control_rt: control_rt,
        }
    }
}
