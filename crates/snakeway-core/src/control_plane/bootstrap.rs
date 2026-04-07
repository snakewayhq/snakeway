use crate::control_plane::acme::{
    CertManager, CertStore, FilesystemCertStore, FilesystemOrderStore, MemoryCertStore, OrderStore,
};
use crate::control_plane::observability::Metrics;
use crate::control_plane::pid::write_pid;
use crate::control_plane::reload::{ReloadEvent, ReloadHandle};
use crate::control_plane::{observability, pid};
use crate::data_plane::bootstrap::build_pingora_server;
use crate::data_plane::ws_connection_management::WsConnectionManager;
use crate::execution::traffic::{TrafficManager, TrafficSnapshot};
use crate::runtime::{ReloadError, RuntimeState, build_runtime_state, reload_runtime_state};
use anyhow::Result;
use arc_swap::ArcSwap;
use nix::NixPath;
use pingora::server::Server;
use snakeway_conf::types::{CertStoreConfig, ListenerConfig, RuntimeConfig, TlsAutomationConfig};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use tracing::{error, info, warn};

/// A fully configured Snakeway server, ready to run.
///
/// Separates server setup (`build`) from execution (`run_blocking` /
/// `run_background`) so that both production and test code share the
/// same initialization path.
pub struct SnakewayServer {
    config: RuntimeConfig,
    config_path: Option<PathBuf>,
    state: Arc<ArcSwap<RuntimeState>>,
    traffic_manager: Arc<TrafficManager>,
    reload: Arc<ReloadHandle>,
    cert_manager: Option<Arc<CertManager>>,
    control_rt: tokio::runtime::Runtime,
    pingora_server: Server,
}

/// Handle to a server running in a background thread.
///
/// Holds the control-plane Tokio runtime so that spawned tasks (reload
/// loop, cert reconciliation) stay alive for the lifetime of the server.
pub struct RuntimeServer {
    pub reload: Arc<ReloadHandle>,
    _control_rt: tokio::runtime::Runtime,
}

impl SnakewayServer {
    /// Build a fully configured server without starting it.
    ///
    /// When `config_path` is `Some`, the reload loop will re-read config
    /// from that directory. When `None`, reload is not supported (typical
    /// for tests using `ConfigBuilder`).
    pub fn build(config_path: Option<PathBuf>, config: RuntimeConfig) -> Result<Self> {
        Self::build_inner(config_path, config, None)
    }

    /// Like `build`, but uses the provided `Metrics` instance instead of
    /// creating one from the telemetry provider. Used by integration tests
    /// that need an in-memory metric exporter.
    pub fn build_with_metrics(
        config_path: Option<PathBuf>,
        config: RuntimeConfig,
        metrics: Arc<Metrics>,
    ) -> Result<Self> {
        Self::build_inner(config_path, config, Some(metrics))
    }

    fn build_inner(
        config_path: Option<PathBuf>,
        config: RuntimeConfig,
        metrics_override: Option<Arc<Metrics>>,
    ) -> Result<Self> {
        use tokio::runtime::Builder;

        let control_rt = Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("failed to build control-plane Tokio runtime");

        // Metrics override is used by tests that provide their own
        // InMemoryMetricExporter. Telemetry and logging initialization
        // happens in run_blocking() for production, not here, to avoid
        // conflicting with test tracing subscribers.
        let metrics = metrics_override;

        // Cert manager (if TLS automation is configured).
        let cert_manager: Option<Arc<CertManager>> = {
            let has_tls = config.listeners.iter().any(|l| l.tls_termination.is_some());
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
            }
        };

        // Initial runtime state.
        let initial_state = build_runtime_state(&config, &cert_manager)?;
        if let (Some(manager), Some(tls)) = (cert_manager.as_ref(), initial_state.tls.as_ref()) {
            manager.attach_tls_sni_map(tls.sni_map.clone());
        }
        let state = Arc::new(ArcSwap::from_pointee(initial_state));
        let traffic_manager = Arc::new(TrafficManager::new(TrafficSnapshot::from_runtime(
            state.load().as_ref(),
        )));

        // Shared infrastructure.
        let reload = Arc::new(ReloadHandle::new());
        let connection_manager = Arc::new(WsConnectionManager::new());

        // Pingora data plane.
        let pingora_server = build_pingora_server(
            config.clone(),
            state.clone(),
            Arc::clone(&traffic_manager),
            Arc::clone(&connection_manager),
            cert_manager.clone(),
            reload.clone(),
            metrics.clone(),
        )
        .map_err(|e| {
            error!(error = %e, "failed to build Pingora server");
            e
        })?;

        Ok(Self {
            config,
            config_path,
            state,
            traffic_manager,
            reload,
            cert_manager,
            control_rt,
            pingora_server,
        })
    }

    /// Run the server, blocking until Pingora exits (production-mode).
    ///
    /// Spawns the signal handler, reload loop, and cert manager
    /// reconciliation on the control-plane runtime before handing off to
    /// Pingora's blocking run loop.
    pub fn run_blocking(self) -> Result<()> {
        // PID file.
        if !self.config.server.pid_file.is_empty() {
            let pid_file = self.config.server.pid_file.clone();
            if let Err(e) = write_pid(&pid_file) {
                warn!(error = %e, pid_file = %pid_file.display(), "failed to write pid file; continuing");
            } else {
                info!(pid_file = %pid_file.display(), "pid file written");
            }
        }

        // Signal handler (SIGHUP -> reload).
        self.control_rt.spawn({
            let reload = self.reload.clone();
            async move {
                let _ = reload.install_signal_handler().await;
            }
        });

        self.spawn_control_plane_tasks();

        // Block on Pingora.
        self.pingora_server.run(Default::default());

        // Cleanup.
        observability::shutdown();

        if !self.config.server.pid_file.is_empty() {
            info!("shutdown requested, removing pid file");
            pid::remove_pid(&self.config.server.pid_file);
        }

        Ok(())
    }

    /// Run the server in a background thread (test-mode).
    ///
    /// Spawns the reload loop and cert manager reconciliation but does
    /// NOT install the signal handler (tests trigger reloads
    /// programmatically, not via SIGHUP).
    pub fn run_background(self) -> RuntimeServer {
        let reload = self.reload.clone();

        self.spawn_control_plane_tasks();

        // Move only the Pingora server into the background thread.
        // The control_rt must stay alive separately so its spawned
        // tasks (reload loop, cert reconciliation) continue to run.
        let server = self.pingora_server;
        let control_rt = self.control_rt;

        thread::spawn(move || {
            server.run_forever();
        });

        RuntimeServer {
            reload,
            _control_rt: control_rt,
        }
    }

    /// Spawn the reload loop and cert manager reconciliation on the
    /// control-plane runtime. Shared between `run_blocking` and
    /// `run_background`.
    fn spawn_control_plane_tasks(&self) {
        // Reload loop (only when a config path is available).
        if let Some(config_path) = &self.config_path {
            self.control_rt.spawn({
                let mut reload_rx = self.reload.subscribe();
                let mut last_epoch = 0;
                let state = self.state.clone();
                let config_path = config_path.clone();
                let traffic = Arc::clone(&self.traffic_manager);
                let cert_manager_for_reload = self.cert_manager.clone();

                async move {
                    info!("Reload loop started");

                    loop {
                        let _ = reload_rx.changed().await;
                        info!("Reload requested");

                        let ReloadEvent { epoch } = *reload_rx.borrow();
                        if epoch <= last_epoch {
                            continue;
                        }

                        last_epoch = epoch;

                        match reload_runtime_state(&config_path, &state, &cert_manager_for_reload)
                            .await
                        {
                            Ok(reloaded_runtime_cfg) => {
                                info!("reload successful");

                                if let Some(manager) = &cert_manager_for_reload {
                                    manager.reload(Arc::new(reloaded_runtime_cfg.clone()));
                                }

                                let new_snapshot =
                                    TrafficSnapshot::from_runtime(state.load().as_ref());
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
        }

        // Cert manager reconciliation.
        if let Some(manager) = &self.cert_manager {
            self.control_rt.spawn(manager.clone().run_reconciliation());
        }
    }
}

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
        .block_on(observability::init_telemetry(&config))
        .unwrap_or_else(|err| {
            warn!("failed to initialize telemetry: {}", err);
            None
        });

    let metrics = telemetry_providers.as_ref().map(|p| Arc::clone(&p.metrics));

    observability::init_logging(telemetry_providers);
    drop(init_rt);

    let server = match metrics {
        Some(m) => SnakewayServer::build_with_metrics(Some(config_path.into()), config, m)?,
        None => SnakewayServer::build(Some(config_path.into()), config)?,
    };

    server.run_blocking()
}

fn build_cert_store(tls_automation_cfg: &TlsAutomationConfig) -> Result<Arc<dyn CertStore>> {
    match &tls_automation_cfg.cert_store {
        CertStoreConfig::Filesystem { cert_dir } => {
            Ok(Arc::new(FilesystemCertStore::new(PathBuf::from(cert_dir))))
        }
        CertStoreConfig::Memory => Ok(Arc::new(MemoryCertStore::default())),
    }
}

fn build_order_store(tls_automation_cfg: &TlsAutomationConfig) -> Result<Arc<dyn OrderStore>> {
    let order_store_dir = tls_automation_cfg.acme.data_dir.clone();
    Ok(Arc::new(FilesystemOrderStore::new(order_store_dir)))
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
