use crate::cert_manager::{CertManager, CertStore, FilesystemCertStore, MemoryCertStore};
use crate::conf::RuntimeConfig;
use crate::conf::types::{CertStoreConfig, ListenerConfig, TlsServerConfig};
use crate::device::core::registry::DeviceRegistry;
use crate::net::{ConnectionRateLimitingFilter, NetworkConnectionFilter};
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

/// Run the Pingora server with the given configuration.
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

                match reload_runtime_state(&config_path, &state).await {
                    Ok(_) => {
                        info!("reload successful");
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

    // Setup WS Connection Manager
    let connection_manager = Arc::new(WsConnectionManager::new());

    // Setup Cert Store and Manager
    let has_tls = config.listeners.iter().any(|l| l.tls.is_some());
    let cert_store = if has_tls && let Some(tls) = &config.server.tls {
        let store = build_cert_store(tls)?;
        let mut manager = CertManager::new(store.clone(), tls.renew_within_days);
        manager.start(&control_rt, Arc::new(config.clone()));
        Some(store)
    } else {
        None
    };

    // Build Pingora server (Pingora owns its own runtimes)
    let server = build_pingora_server(
        config.clone(),
        state,
        Arc::clone(&traffic_manager),
        Arc::clone(&connection_manager),
        reload.clone(),
        cert_store,
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

fn build_cert_store(tls_server_cfg: &TlsServerConfig) -> Result<Arc<dyn CertStore>> {
    match &tls_server_cfg.cert_store {
        CertStoreConfig::Filesystem(cert_dir) => {
            // Attempt to create the cert store dir if it doesn't exist.
            std::fs::create_dir_all(&cert_dir)
                .map_err(|e| anyhow!("failed to create cert store dir: {}", e))?;
            Ok(Arc::new(FilesystemCertStore::new(PathBuf::from(cert_dir))))
        }
        CertStoreConfig::Memory => Ok(Arc::new(MemoryCertStore::new())),
    }
}

/// Build the Pingora server.
pub fn build_pingora_server(
    config: RuntimeConfig,
    state: Arc<ArcSwap<RuntimeState>>,
    traffic_manager: Arc<TrafficManager>,
    connection_manager: Arc<WsConnectionManager>,
    reload: Arc<ReloadHandle>,
    maybe_cert_store: Option<Arc<dyn CertStore>>,
) -> Result<Server, Error> {
    let mut pingora_server_conf =
        ServerConf::new().expect("Could not construct pingora server configuration");
    if !config.server.ca_file.is_empty() {
        pingora_server_conf.ca_file = Some(config.server.ca_file.clone());
    }

    pingora_server_conf.work_stealing = config.server.work_stealing;

    let mut server = if let Some(threads) = config.server.threads {
        debug!(
            threads,
            "Creating Pingora server with overridden worker threads"
        );
        pingora_server_conf.threads = threads;
        Server::new_with_opt_and_conf(None, pingora_server_conf)
    } else {
        // Create a Pingora server with default settings.
        // "None" is required here to truly tell Pingora to use its default settings.
        Server::new(None)?
    };

    server.bootstrap();

    // Load devices
    let mut registry = DeviceRegistry::new();
    registry.load_from_config(&config)?;
    debug!("Loaded device count = {}", registry.all().len());

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

        match &listener_cfg.tls {
            Some(tls) => {
                if let Some(static_options) = &tls.static_options {
                    let callbacks = build_tls_callbacks(CertMode::Static);
                    let mut tls_settings = TlsSettings::with_callbacks(callbacks)?;
                    tls_settings.set_private_key_file(&static_options.key, SslFiletype::PEM)?;
                    tls_settings.set_certificate_chain_file(&static_options.cert)?;
                    if listener_cfg.enable_http2 {
                        tls_settings.enable_h2();
                    }
                    public_svc.add_tls_with_settings(
                        &listener_cfg.addr.to_string(),
                        None,
                        tls_settings,
                    );
                } else if let Some(acme_options) = &tls.acme_options
                    && let Some(ref cert_store) = maybe_cert_store
                {
                    let callbacks = build_tls_callbacks(CertMode::Acme(cert_store.clone()));
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
            }
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

    // Create redirect listener(s).
    for listener_cfg in config
        .listeners
        .iter()
        .filter(|l| !l.enable_admin && l.redirect.is_some())
    {
        if let Some(redirect) = &listener_cfg.redirect {
            // Build and register the redirect Pingora HTTP proxy service with a standalone listener.
            let redirect_gateway =
                RedirectGateway::new(redirect.destination.clone(), redirect.response_code);

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

    // Build the admin HTTP proxy service from Pingora.
    for listener_cfg in config.listeners.iter().filter(|l| l.enable_admin) {
        if let Some(tls) = &listener_cfg.tls {
            let admin_gateway = AdminGateway::new(
                traffic_manager.clone(),
                connection_manager.clone(),
                reload.clone(),
            );
            let mut admin_svc = http_proxy_service(&server.configuration, admin_gateway);

            match &listener_cfg.tls {
                Some(tls) => {
                    if let Some(static_options) = &tls.static_options {
                        let callbacks = build_tls_callbacks(CertMode::Static);
                        let mut tls_settings = TlsSettings::with_callbacks(callbacks)?;
                        tls_settings.set_private_key_file(&static_options.key, SslFiletype::PEM)?;
                        tls_settings.set_certificate_chain_file(&static_options.cert)?;

                        admin_svc.add_tls_with_settings(
                            &listener_cfg.addr.to_string(),
                            None,
                            tls_settings,
                        );
                    } else if let Some(acme_options) = &tls.acme_options
                        && let Some(ref cert_store) = maybe_cert_store
                    {
                        let callbacks = build_tls_callbacks(CertMode::Acme(cert_store.clone()));
                        let tls_settings = TlsSettings::with_callbacks(callbacks)?;
                        admin_svc.add_tls_with_settings(
                            &listener_cfg.addr.to_string(),
                            None,
                            tls_settings,
                        );
                    }
                }
                None => {
                    admin_svc.add_tcp(&listener_cfg.addr.to_string());
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
