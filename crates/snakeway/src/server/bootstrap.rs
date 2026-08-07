use crate::server::ControlPlaneServer;
use anyhow::{Context, Result};
use snakeway_conf::types::{ListenerConfig, RuntimeConfig};
use snakeway_observability::{init_logging, init_telemetry};
use std::net::TcpListener;
use std::sync::Arc;
use tracing::{error, warn};

/// Convenience wrapper that builds and runs the server in blocking mode.
/// This is the production entry point called by `start_server()`.
pub fn start_control_plane(config_path: &str, config: RuntimeConfig, upgrade: bool) -> Result<()> {
    if !upgrade {
        bail_if_port_is_in_use(&config.listeners)?;
    }

    // Initialize telemetry and logging before building the server so
    // that metrics are available for the Pingora data plane.
    // This must happen here (not in build()) because tests manage their own
    // tracing subscriber and would conflict with init_logging.
    use tokio::runtime::Builder;
    let init_rt = Builder::new_current_thread()
        .enable_all()
        .build()
        .context("failed to build init runtime")?;

    let telemetry_providers = init_rt
        .block_on(init_telemetry(&config))
        .unwrap_or_else(|err| {
            warn!("failed to initialize telemetry: {}", err);
            None
        });

    let metrics = telemetry_providers.as_ref().map(|p| Arc::clone(&p.metrics));

    init_logging(telemetry_providers);

    // Safe to drop.
    // init_telemetry only returns owned providers and does not spawn tasks
    // that depend on this runtime staying alive.
    drop(init_rt);

    let server = match metrics {
        Some(m) => {
            ControlPlaneServer::build_with_metrics(Some(config_path.into()), config, m, upgrade)?
        }
        None => ControlPlaneServer::build(Some(config_path.into()), config, upgrade)?,
    };

    server.run_blocking()
}

fn bail_if_port_is_in_use(listeners: &[ListenerConfig]) -> Result<()> {
    let mut has_error = false;
    for cfg in listeners.iter() {
        if TcpListener::bind(&cfg.addr).is_err() {
            error!("Listener {} ({}) already in use", cfg.name, cfg.addr);
            has_error = true;
        }
    }
    if has_error {
        anyhow::bail!("One or more listeners are already in use");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use confval::prelude::{Located, Report};
    use snakeway_conf::types::BindSpec;

    fn listener_on(port: u16) -> ListenerConfig {
        let spec = BindSpec {
            interface: Located::detached("loopback".to_string()),
            port: Located::detached(i64::from(port)),
            ..Default::default()
        };
        ListenerConfig::from_bind("test-listener", &spec, &mut Report::new())
            .expect("listener config")
    }

    #[test]
    fn should_bail_when_a_listener_port_is_in_use() {
        // Arrange
        let occupied = TcpListener::bind("127.0.0.1:0").expect("bind probe");
        let port = occupied.local_addr().expect("local addr").port();
        let listeners = vec![listener_on(port)];

        // Act
        let result = bail_if_port_is_in_use(&listeners);

        // Assert
        let err = result.expect_err("an occupied port must bail");
        assert!(err.to_string().contains("already in use"));
    }

    #[test]
    fn should_pass_when_listener_ports_are_free() {
        // Arrange
        let probe = TcpListener::bind("127.0.0.1:0").expect("bind probe");
        let port = probe.local_addr().expect("local addr").port();
        drop(probe);
        let listeners = vec![listener_on(port)];

        // Act
        let result = bail_if_port_is_in_use(&listeners);

        // Assert
        assert!(result.is_ok());
    }
}
