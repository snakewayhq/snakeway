use crate::conf::RuntimeConfig;
use crate::device::core::registry::DeviceRegistry;
use crate::server::pid;
use crate::server::proxy::SnakewayGateway;
use crate::server::reload::ReloadHandle;
use crate::server::runtime::{RuntimeState, build_runtime_state, reload_runtime_state};
use anyhow::{Error, Result};
use arc_swap::ArcSwap;
use pingora::prelude::*;
use pingora::server::Server;
use pingora::server::configuration::ServerConf;
use std::path::PathBuf;
use std::sync::Arc;

/// Run the Pingora server with the given configuration.
pub fn run(config_path: String, config: RuntimeConfig) -> Result<()> {
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
            let _ = reload.install_signal_handler().await;
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
    config: RuntimeConfig,
    state: Arc<ArcSwap<RuntimeState>>,
) -> Result<Server, Error> {
    let mut server = if let Some(threads) = config.server.threads {
        tracing::debug!(
            threads,
            "Creating Pingora server with overridden worker threads"
        );
        let mut conf = ServerConf::new().expect("Could not construct pingora server configuration");
        conf.threads = threads;
        Server::new_with_opt_and_conf(None, conf)
    } else {
        // Create a Pingora server with default settings.
        // "None" is required here to truly tell Pingora to use its default settings.
        Server::new(None)?
    };

    server.bootstrap();

    // Load devices
    let mut registry = DeviceRegistry::new();
    registry.load_from_config(&config)?;
    tracing::debug!("Loaded device count = {}", registry.all().len());

    // Build gateway
    let gateway = SnakewayGateway {
        state: state.clone(),
    };

    // Build HTTP proxy service from Pingora.
    let mut svc = http_proxy_service(&server.configuration, gateway);
    if let Some(tls) = &config.server.tls {
        svc.add_tls(&config.server.listen, &tls.cert, &tls.key)?;
    } else {
        svc.add_tcp(&config.server.listen);
    }

    // Register service.
    server.add_service(svc);

    Ok(server)
}
