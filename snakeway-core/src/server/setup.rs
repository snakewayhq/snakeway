use crate::config::SnakewayConfig;
use crate::device::core::registry::DeviceRegistry;
use crate::server::pid;
use crate::server::proxy::SnakewayGateway;
use crate::server::reload::ReloadHandle;
use crate::server::runtime::{RuntimeState, build_runtime_state, reload_runtime_state};
use anyhow::{Error, Result, anyhow};
use arc_swap::ArcSwap;
use pingora::prelude::*;
use pingora::server::Server;
use std::path::PathBuf;
use std::sync::Arc;

/// Run the Pingora server with the given configuration.
pub fn run(config_path: String, config: SnakewayConfig) -> Result<()> {
    use tokio::runtime::Builder;

    let config_path = PathBuf::from(config_path);

    // Attempt to write pid file (best-effort)
    if let Some(pid_file) = &config.server.pid_file {
        if let Err(e) = pid::write_pid(pid_file) {
            tracing::warn!(error = %e, pid_file, "failed to write pid file; continuing");
        } else {
            tracing::info!(pid_file, "pid file written");
        }
    }

    // Build initial runtime state (reloadable)
    let initial_state = build_runtime_state(&config)?;
    let state = Arc::new(ArcSwap::from_pointee(initial_state));

    // Control-plane runtime (signals + reload only)
    let control_rt = Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("failed to build control-plane Tokio runtime");

    // Reload wiring
    let reload = ReloadHandle::new();

    // Spawn signal handler
    control_rt.spawn({
        let reload = reload.clone();
        async move {
            reload.install_signal_handler().await;
        }
    });

    // Spawn reload loop
    control_rt.spawn({
        let mut reload_rx = reload.subscribe();
        let state = state.clone();
        let config_path = config_path.clone();

        async move {
            tracing::info!("Reload loop started");

            loop {
                let _ = reload_rx.changed().await;
                tracing::info!("Reload requested");

                match reload_runtime_state(&config_path, &state).await {
                    Ok(_) => tracing::info!("reload successful"),
                    Err(e) => tracing::error!(error = %e, "reload failed"),
                }
            }
        }
    });

    // Build Pingora server (Pingora owns its own runtimes)
    let server = build_pingora_server(config.clone(), state)?;

    // Ensure pid file cleanup on shutdown
    if let Some(pid_file) = config.server.pid_file.clone() {
        ctrlc::set_handler(move || {
            tracing::info!("shutdown requested, removing pid file");
            pid::remove_pid(&pid_file);
            std::process::exit(0);
        })?;
    }

    // IMPORTANT:
    // - control_rt must stay in scope so its worker thread lives
    // - run_forever blocks the main thread as intended
    server.run_forever();
}

/// Build the Pingora server.
pub fn build_pingora_server(
    config: SnakewayConfig,
    state: Arc<ArcSwap<RuntimeState>>,
) -> Result<Server, Error> {
    let mut server = Server::new(None)?;
    server.bootstrap();

    // Load devices
    let mut registry = DeviceRegistry::new();
    registry.load_from_config(&config)?;
    tracing::debug!("Loaded device count = {}", registry.all().len());

    // Extract upstream route (exactly one required)
    let upstream_route = config
        .routes
        .iter()
        .find(|r| r.upstream.is_some())
        .ok_or_else(|| anyhow!("Snakeway: exactly one upstream route is required"))?;

    let (host, port) = parse_upstream(
        upstream_route
            .upstream
            .as_ref()
            .expect("validated upstream"),
    )?;

    // Build gateway
    let gateway = SnakewayGateway {
        upstream_host: host,
        upstream_port: port,
        use_tls: false,
        sni: String::new(),
        state: state.clone(),
    };

    // Build HTTP proxy service from Pingora.
    let mut svc = http_proxy_service(&server.configuration, gateway);
    svc.add_tcp(&config.server.listen);

    // Register service.
    server.add_service(svc);

    Ok(server)
}

/// Parse an upstream address of the form "host:port".
fn parse_upstream(s: &str) -> Result<(String, u16)> {
    let mut parts = s.split(':');
    let host = parts
        .next()
        .ok_or_else(|| anyhow!("invalid upstream address: {}", s))?;
    let port = parts
        .next()
        .ok_or_else(|| anyhow!("invalid upstream address: {}", s))?
        .parse::<u16>()?;

    Ok((host.to_string(), port))
}
