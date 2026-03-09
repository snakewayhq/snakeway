use crate::control_plane::acme::{
    CertManager, CertStore, FilesystemCertStore, FilesystemOrderStore, MemoryCertStore, OrderStore,
};
use crate::control_plane::pid::write_pid;
use crate::control_plane::reload::{ReloadEvent, ReloadHandle};
use crate::control_plane::{observability, pid};
use crate::data_plane::bootstrap::build_pingora_server;
use crate::data_plane::ws_connection_management::WsConnectionManager;
use crate::execution::traffic::{TrafficManager, TrafficSnapshot};
use crate::runtime::{ReloadError, build_runtime_state, reload_runtime_state};
use anyhow::{Result, anyhow};
use arc_swap::ArcSwap;
use nix::NixPath;
use snakeway_conf::types::{CertStoreConfig, ListenerConfig, RuntimeConfig, TlsAutomationConfig};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{error, info, warn};

/// Start the Snakeway control plane, the start the Pingora data plane.
pub fn start_control_plane(config_path: &str, config: RuntimeConfig) -> Result<()> {
    bail_if_port_is_in_use(&config.listeners)?;

    use tokio::runtime::Builder;

    let config_path = PathBuf::from(config_path);

    // Attempt to write pid file (best-effort)
    if !&config.server.pid_file.is_empty() {
        let pid_file = config.server.pid_file.clone();
        if let Err(e) = write_pid(&pid_file) {
            warn!(error = %e, pid_file = %pid_file.display(), "failed to write pid file; continuing");
        } else {
            info!(pid_file = %pid_file.display(), "pid file written");
        }
    }

    // Control-plane runtime (signals and reload only)
    let control_rt = Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("failed to build control-plane Tokio runtime");

    // Load telemetry and logging
    let tracer = control_rt
        .block_on(observability::init_telemetry(&config))
        .unwrap_or_else(|err| {
            tracing::warn!("failed to initialize telemetry: {}", err);
            None
        });

    observability::init_logging(tracer);

    // Set up the Cert Store and Manager.
    let has_tls = config.listeners.iter().any(|l| l.tls_termination.is_some());
    let cert_manager: Option<Arc<CertManager>> =
        if has_tls && let Some(tls_automation_cfg) = &config.server.tls_automation {
            let cert_store = build_cert_store(tls_automation_cfg)?;
            let order_store = build_order_store(tls_automation_cfg)?;
            let manager = Arc::new(CertManager::new(
                cert_store,
                order_store,
                Arc::new(config.clone()),
                tls_automation_cfg,
            ));
            control_rt.block_on(manager.initialize(&tls_automation_cfg.acme))?;
            Some(manager)
        } else {
            None
        };

    // Build initial runtime state (reloadable)
    let initial_state = build_runtime_state(&config, &cert_manager)?;
    if let (Some(manager), Some(tls)) = (cert_manager.as_ref(), initial_state.tls.as_ref()) {
        // Attach the SNI map to the cert manager.
        manager.attach_tls_sni_map(tls.sni_map.clone());
    }
    let state = Arc::new(ArcSwap::from_pointee(initial_state));
    let traffic_manager = Arc::new(TrafficManager::new(TrafficSnapshot::from_runtime(
        state.load().as_ref(),
    )));

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
        let cert_manager_for_reload = cert_manager.clone();

        async move {
            info!("Reload loop started");

            loop {
                let _ = reload_rx.changed().await;
                info!("Reload requested");

                let ReloadEvent { epoch } = *reload_rx.borrow();
                if epoch <= last_epoch {
                    // already handled
                    continue;
                }

                last_epoch = epoch;

                match reload_runtime_state(&config_path, &state, &cert_manager_for_reload).await {
                    Ok(reloaded_runtime_cfg) => {
                        info!("reload successful");

                        // Update the cert manager with the new runtime configuration.
                        // Note: attach_tls_sni_map is called inside reload_runtime_state
                        // before the state swap, so no separate call is needed here.
                        if let Some(manager) = &cert_manager_for_reload {
                            manager.reload(Arc::new(reloaded_runtime_cfg.clone()));
                        }

                        // Generate traffic snapshot.
                        let new_snapshot = TrafficSnapshot::from_runtime(state.load().as_ref());
                        traffic.update(new_snapshot);
                    }
                    Err(reload_err) => match reload_err {
                        ReloadError::Load(e) => {
                            error!(error = %e, "failed to reload config");
                        }
                        ReloadError::InvalidConfig { report } => {
                            error!(
                                error = "configuration validation failed",
                                error_count = report.errors.len(),
                                warning_count = report.warnings.len(),
                                "reload failed"
                            )
                        }
                        ReloadError::Build(e) => {
                            error!(error = %e, "failed to build runtime state");
                        }
                    },
                }
            }
        }
    });

    // Set up WS Connection Manager
    let connection_manager = Arc::new(WsConnectionManager::new());

    // start manager
    if let Some(manager) = &cert_manager {
        control_rt.spawn(manager.clone().run_reconciliation());
    }

    // Build Pingora server (Pingora owns its own runtimes)
    let server = build_pingora_server(
        config.clone(),
        state,
        Arc::clone(&traffic_manager),
        Arc::clone(&connection_manager),
        cert_manager,
        reload.clone(),
    )
    .map_err(|e| {
        error!(error = %e, "failed to build Pingora server");
        e
    })?;

    // Ensure pid file cleanup on shutdown
    if !config.server.pid_file.is_empty() {
        ctrlc::set_handler(move || {
            info!("shutdown requested, removing pid file");
            pid::remove_pid(&config.server.pid_file);
            std::process::exit(0);
        })?;
    }

    // IMPORTANT:
    // - control_rt must stay in scope so its worker thread lives
    // - run_forever blocks the main thread as intended
    server.run_forever();
}

fn build_cert_store(tls_automation_cfg: &TlsAutomationConfig) -> Result<Arc<dyn CertStore>> {
    match &tls_automation_cfg.cert_store {
        CertStoreConfig::Filesystem { cert_dir } => {
            // Attempt to create the cert store dir if it doesn't exist.
            std::fs::create_dir_all(cert_dir)
                .map_err(|e| anyhow!("failed to create cert store dir: {}", e))?;
            Ok(Arc::new(FilesystemCertStore::new(PathBuf::from(cert_dir))))
        }
        CertStoreConfig::Memory => Ok(Arc::new(MemoryCertStore::default())),
    }
}

fn build_order_store(tls_automation_cfg: &TlsAutomationConfig) -> Result<Arc<dyn OrderStore>> {
    let order_store_dir = tls_automation_cfg.acme.data_dir.clone();
    std::fs::create_dir_all(order_store_dir.clone())
        .map_err(|e| anyhow!("failed to create order store dir: {}", e))?;
    Ok(Arc::new(FilesystemOrderStore::new(order_store_dir)))
}

/// Sanity check if ports are already in use by listeners (or something else).
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
