use crate::conf::RuntimeConfig;
use crate::connection_management::ConnectionManager;
use crate::device::core::registry::DeviceRegistry;
use crate::proxy::{AdminGateway, PublicGateway};
use crate::runtime::{RuntimeState, build_runtime_state, reload_runtime_state};
use crate::server::pid;
use crate::server::reload::{ReloadEvent, ReloadHandle};
use crate::traffic_management::{TrafficManager, TrafficSnapshot};
use anyhow::{Error, Result};
use arc_swap::ArcSwap;
use pingora::listeners::tls::TlsSettings;
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
            tracing::warn!(error = %e, pid_file = %pid_file.display(), "failed to write pid file; continuing");
        } else {
            tracing::info!(pid_file = %pid_file.display(), "pid file written");
        }
    }

    // Build initial runtime state (reloadable)
    let initial_state = build_runtime_state(&config)?;
    let state = Arc::new(ArcSwap::from_pointee(initial_state));
    let traffic_manager = Arc::new(TrafficManager::new(TrafficSnapshot::from_runtime(
        state.load().as_ref(),
    )));

    // Control-plane runtime (signals + reload only)
    let control_rt = Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("failed to build control-plane Tokio runtime");

    // Reload wiring
    let reload = Arc::new(ReloadHandle::new());

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
        let mut last_epoch = 0;
        let state = state.clone();
        let config_path = config_path.clone();
        let traffic = Arc::clone(&traffic_manager);

        async move {
            tracing::info!("Reload loop started");

            loop {
                let _ = reload_rx.changed().await;
                tracing::info!("Reload requested");

                let ReloadEvent { epoch } = *reload_rx.borrow();
                if epoch <= last_epoch {
                    // already handled
                    continue;
                }

                last_epoch = epoch;

                match reload_runtime_state(&config_path, &state).await {
                    Ok(_) => {
                        tracing::info!("reload successful");
                        let new_snapshot = TrafficSnapshot::from_runtime(state.load().as_ref());
                        traffic.update(new_snapshot);
                    }
                    Err(e) => tracing::error!(error = %e, "reload failed"),
                }
            }
        }
    });

    let connection_manager = Arc::new(ConnectionManager::new());

    // Build Pingora server (Pingora owns its own runtimes)
    let server = build_pingora_server(
        config.clone(),
        state,
        Arc::clone(&traffic_manager),
        Arc::clone(&connection_manager),
        reload.clone(),
    )
    .map_err(|e| {
        tracing::error!(error = %e, "failed to build Pingora server");
        e
    })?;

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
    traffic_manager: Arc<TrafficManager>,
    connection_manager: Arc<ConnectionManager>,
    reload: Arc<ReloadHandle>,
) -> Result<Server, Error> {
    let mut conf = ServerConf::new().expect("Could not construct pingora server configuration");
    conf.ca_file = config.server.ca_file.clone();

    let mut server = if let Some(threads) = config.server.threads {
        tracing::debug!(
            threads,
            "Creating Pingora server with overridden worker threads"
        );
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

    // Build the public HTTP proxy service from Pingora.
    let public_gateway = PublicGateway::new(
        state.clone(),
        traffic_manager.clone(),
        connection_manager.clone(),
    );
    let mut public_svc = http_proxy_service(&server.configuration, public_gateway);
    for listener in &config
        .listeners
        .iter()
        .filter(|l| !l.enable_admin)
        .collect::<Vec<_>>()
    {
        match &listener.tls {
            Some(tls) => {
                let mut tls_settings = TlsSettings::intermediate(&tls.cert, &tls.key)?;
                if listener.enable_http2 {
                    tls_settings.enable_h2();
                }
                public_svc.add_tls_with_settings(&listener.addr.to_string(), None, tls_settings);
            }
            None => {
                public_svc.add_tcp(&listener.addr.to_string());
            }
        }
    }

    // Register public service.
    server.add_service(public_svc);

    // Build the admin HTTP proxy service from Pingora.
    let admin_gateway = AdminGateway::new(traffic_manager, connection_manager, reload);
    let mut admin_svc = http_proxy_service(&server.configuration, admin_gateway);
    for listener in &config
        .listeners
        .iter()
        .filter(|l| l.enable_admin)
        .collect::<Vec<_>>()
    {
        match &listener.tls {
            Some(tls) => {
                let tls_settings = TlsSettings::intermediate(&tls.cert, &tls.key)?;
                admin_svc.add_tls_with_settings(&listener.addr, None, tls_settings);
            }
            None => {
                admin_svc.add_tcp(&listener.addr);
            }
        }
    }

    // Register admin service.
    server.add_service(admin_svc);

    Ok(server)
}
