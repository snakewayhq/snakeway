use crate::cert_manager::{
    CertManager, CertStore, FilesystemCertStore, FilesystemOrderStore, MemoryCertStore, OrderStore,
};
use crate::conf::types::{CertStoreConfig, ListenerConfig, TlsAutomationConfig};
use crate::conf::{RuntimeConfig, TlsTerminationConfig};
use crate::device::core::registry::DeviceRegistry;
use crate::net::{ConnectionRateLimitingFilter, NetworkConnectionFilter};
use crate::observability;
use crate::proxy::{AdminGateway, PublicGateway, RedirectGateway};
use crate::runtime::{ReloadError, RuntimeState, build_runtime_state, reload_runtime_state};
use crate::server::pid;
use crate::server::reload::{ReloadEvent, ReloadHandle};
use crate::server::tls_handshake::{CertMode, build_tls_callbacks};
use crate::traffic_management::{TrafficManager, TrafficSnapshot};
use crate::ws_connection_management::WsConnectionManager;
use anyhow::{Error, Result, anyhow};
use arc_swap::ArcSwap;
use nix::NixPath;
use openssl::ssl::SslFiletype;
use pingora::listeners::tls::TlsSettings;
use pingora::prelude::*;
use pingora::server::Server;
use pingora::server::configuration::ServerConf;
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// Start the Snakeway control plane, the start the Pingora data plane.
pub fn run(config_path: &str, config: RuntimeConfig) -> Result<()> {
    bail_if_port_is_in_use(&config.listeners)?;

    use tokio::runtime::Builder;

    let config_path = PathBuf::from(config_path);

    // Attempt to write pid file (best-effort)
    if !&config.server.pid_file.is_empty() {
        let pid_file = config.server.pid_file.clone();
        if let Err(e) = pid::write_pid(&pid_file) {
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
    let tracer = control_rt.block_on(async { observability::init_telemetry(&config) });
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

/// Build the Pingora server.
///
/// There are three types of proxy services constructed:
///
/// 1. Public: Services defined in ingress.d/* configuration files.
/// 2. Redirect: Services created from optional redirect settings in ingress file bind blocks.
/// 3. Admin: The Snakeway Admin API
pub fn build_pingora_server(
    config: RuntimeConfig,
    state: Arc<ArcSwap<RuntimeState>>,
    traffic_manager: Arc<TrafficManager>,
    connection_manager: Arc<WsConnectionManager>,
    cert_manager: Option<Arc<CertManager>>,
    reload: Arc<ReloadHandle>,
) -> Result<Server, Error> {
    let mut pingora_server_conf =
        ServerConf::new().expect("Could not construct pingora server configuration");

    pingora_server_conf.ca_file = config.server.ca_file.clone();
    pingora_server_conf.work_stealing = config.server.work_stealing;

    if let Some(threads) = config.server.threads {
        debug!(
            threads,
            "Creating Pingora server with overridden worker threads"
        );
        pingora_server_conf.threads = threads;
    }
    let mut server = Server::new_with_opt_and_conf(None, pingora_server_conf);

    server.bootstrap();

    // Load devices
    let mut registry = DeviceRegistry::new();
    registry.load_from_config(&config)?;
    debug!("Loaded device count = {}", registry.all().len());

    //-------------------------------------------------------------------------
    // Public Proxy: Create public listener(s).
    //-------------------------------------------------------------------------
    for listener_cfg in config
        .listeners
        .iter()
        .filter(|l| !l.enable_admin && l.redirect.is_none())
    {
        // Build the public HTTP proxy service from Pingora.
        let public_gateway = PublicGateway::new(
            Arc::from(listener_cfg.name.clone()),
            state.clone(),
            traffic_manager.clone(),
            connection_manager.clone(),
        );
        let mut public_svc = http_proxy_service(&server.configuration, public_gateway);

        match &listener_cfg.tls_termination {
            Some(certificate_cfg) => match certificate_cfg {
                TlsTerminationConfig::Manual { key, cert } => {
                    let callbacks = build_tls_callbacks(CertMode::Manual);
                    let mut tls_settings = TlsSettings::with_callbacks(callbacks)?;
                    tls_settings.set_private_key_file(key, SslFiletype::PEM)?;
                    tls_settings.set_certificate_chain_file(cert)?;
                    if listener_cfg.enable_http2 {
                        tls_settings.enable_h2();
                    }
                    public_svc.add_tls_with_settings(
                        &listener_cfg.addr.to_string(),
                        None,
                        tls_settings,
                    );
                }
                TlsTerminationConfig::Acme { .. } => {
                    let callbacks = build_tls_callbacks(CertMode::Acme(state.clone()));
                    let mut tls_settings = TlsSettings::with_callbacks(callbacks)?;
                    if listener_cfg.enable_http2 {
                        tls_settings.enable_h2();
                    }
                    public_svc.add_tls_with_settings(
                        &listener_cfg.addr.to_string(),
                        None,
                        tls_settings,
                    );
                }
            },
            None => {
                public_svc.add_tcp(&listener_cfg.addr.to_string());
            }
        }

        if let Some(connection_filter_cfg) = &listener_cfg.connection_filter {
            public_svc.set_connection_filter(Arc::new(NetworkConnectionFilter::from(
                connection_filter_cfg.clone(),
            )));
        }

        if let Some(rate_limiting_filter_cfg) = &listener_cfg.connection_rate_limiting_filter {
            public_svc.set_connection_filter(Arc::new(ConnectionRateLimitingFilter::from(
                rate_limiting_filter_cfg.clone(),
            )));
        }

        // Register public service.
        server.add_service(public_svc);
    }

    //-------------------------------------------------------------------------
    // Redirect Proxy: Create redirect listener(s).
    //-------------------------------------------------------------------------
    for listener_cfg in config
        .listeners
        .iter()
        .filter(|l| !l.enable_admin && l.redirect.is_some())
    {
        if let Some(redirect) = &listener_cfg.redirect {
            // Build and register the redirect Pingora HTTP proxy service with a standalone listener.
            let redirect_gateway = RedirectGateway::new(
                redirect.destination.clone(),
                redirect.response_code,
                cert_manager.clone(),
            );

            // Create a TCP listener for the redirect service.
            let mut redirect_scv = http_proxy_service(&server.configuration, redirect_gateway);
            redirect_scv.add_tcp(&listener_cfg.addr);

            // Add a connection filter if configured.
            if let Some(connection_filter_cfg) = &listener_cfg.connection_filter {
                redirect_scv.set_connection_filter(Arc::new(NetworkConnectionFilter::from(
                    connection_filter_cfg.clone(),
                )));
            }
            server.add_service(redirect_scv);
        }
    }

    //-------------------------------------------------------------------------
    // Admin Proxy: Create the admin API listener(s).
    //-------------------------------------------------------------------------
    for listener_cfg in config.listeners.iter().filter(|l| l.enable_admin) {
        if let Some(certificate_cfg) = &listener_cfg.tls_termination {
            let admin_gateway = AdminGateway::new(
                traffic_manager.clone(),
                connection_manager.clone(),
                reload.clone(),
                cert_manager.clone(),
            );
            let mut admin_svc = http_proxy_service(&server.configuration, admin_gateway);

            match certificate_cfg {
                TlsTerminationConfig::Manual { key, cert } => {
                    // This may seem a little dicey, but the configuration layer validates the file
                    // pair and reports errors long before this code is ever called.
                    // If these errors are produced, it means there is a bug in validation,
                    // or the cert files were deleted a microsecond between validation and use.
                    let cert_str = cert
                        .to_str()
                        .ok_or_else(|| anyhow!("Certificate path is not valid UTF-8"))?;
                    let key_str = key
                        .to_str()
                        .ok_or_else(|| anyhow!("Key path is not valid UTF-8"))?;
                    let tls_settings = TlsSettings::intermediate(cert_str, key_str)?;
                    admin_svc.add_tls_with_settings(&listener_cfg.addr, None, tls_settings);
                }
                TlsTerminationConfig::Acme { .. } => {
                    // ACME is not supported for admin API.
                    return Err(anyhow!(
                        "ACME TLS is not supported for admin API. This is a bug as it should have been caught by validation."
                    ));
                }
            }

            // Register admin service.
            server.add_service(admin_svc);
        } else {
            warn!(
                "Admin API listener {} has no TLS configured and will not be bound",
                listener_cfg.name
            );
        }
    }

    Ok(server)
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
