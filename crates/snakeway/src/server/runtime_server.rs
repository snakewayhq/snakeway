use snakeway_proxy::ReloadHandle;
use std::sync::Arc;

/// Handle to a server running in a background thread.
///
/// Holds the control-plane Tokio runtime so that spawned tasks (reload
/// loop, cert reconciliation) stay alive for the lifetime of the server.
pub struct RuntimeServer {
    pub reload: Arc<ReloadHandle>,
    /// Holds the control-plane runtime open for the lifetime of this handle, so the
    /// reload loop and cert reconciliation keep running. Nothing reads the field, and
    /// dropping it shuts the runtime down.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_construct_runtime_server() {
        // Arrange
        let reload = Arc::new(ReloadHandle::new());
        let control_rt =
            tokio::runtime::Runtime::new().expect("Cannot create tokio runtime in test");

        // Act
        let result = RuntimeServer::new(reload.clone(), control_rt);

        // Assert
        assert!(Arc::ptr_eq(&result.reload, &reload));
    }
}
