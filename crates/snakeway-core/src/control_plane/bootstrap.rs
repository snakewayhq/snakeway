use crate::control_plane::ControlPlaneServer;
use crate::control_plane::observability::{init_logging, init_telemetry};
use anyhow::Result;
use snakeway_conf::types::{ListenerConfig, RuntimeConfig};
use std::net::TcpListener;
use std::sync::Arc;
use tracing::{error, warn};

/// Convenience wrapper that builds and runs the server in blocking mode.
/// This is the production entry point called by `start_server()`.
pub fn start_control_plane(config_path: &str, config: RuntimeConfig) -> Result<()> {
    bail_if_port_is_in_use(&config.listeners)?;

    // Initialize telemetry and logging before building the server so
    // that metrics are available for the Pingora data plane.
    // This must happen here (not in build()) because tests manage their own
    // tracing subscriber and would conflict with init_logging.
    use tokio::runtime::Builder;
    let init_rt = Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to build init runtime");

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
        Some(m) => ControlPlaneServer::build_with_metrics(Some(config_path.into()), config, m)?,
        None => ControlPlaneServer::build(Some(config_path.into()), config)?,
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
