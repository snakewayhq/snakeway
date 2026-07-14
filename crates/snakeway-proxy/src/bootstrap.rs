use crate::proxy::{AdminGateway, PublicGateway, RedirectGateway};
use crate::reload::ReloadHandle;
use crate::tls_handshake::{CertMode, build_tls_callbacks};
use anyhow::{Error, Result, anyhow};
use arc_swap::ArcSwap;
use openssl::ssl::SslFiletype;
use pingora::listeners::tls::TlsSettings;
use pingora::prelude::*;
use pingora::protocols::http::v2::server::default_h2_options;
use pingora::server::Server;
use pingora::server::configuration::{Opt, ServerConf};
use snakeway_acme::CertManager;
use snakeway_conf::types::{RuntimeConfig, TlsTerminationConfig};
use snakeway_engine::WsConnectionManager;
use snakeway_engine::runtime::RuntimeState;
use snakeway_engine::traffic::TrafficManager;
use snakeway_net::{ConnectionRateLimitingFilter, NetworkConnectionFilter};
use snakeway_observability::Metrics;
use std::sync::Arc;
use tracing::{debug, warn};

pub struct DataPlaneServerParams {
    pub config: RuntimeConfig,
    pub state: Arc<ArcSwap<RuntimeState>>,
    pub traffic_manager: Arc<TrafficManager>,
    pub connection_manager: Arc<WsConnectionManager>,
    pub cert_manager: Option<Arc<CertManager>>,
    pub reload: Arc<ReloadHandle>,
    pub metrics: Option<Arc<Metrics>>,
    pub upgrade: bool,
}

/// Build the Pingora server.
///
/// There are three types of proxy services constructed:
///
/// 1. Public: Services defined in ingress.d/* configuration files.
/// 2. Redirect: Services created from optional redirect settings in ingress file bind blocks.
/// 3. Admin: The Snakeway Admin API
pub fn build_pingora_server(params: DataPlaneServerParams) -> Result<Server, Error> {
    let DataPlaneServerParams {
        config,
        state,
        traffic_manager,
        connection_manager,
        cert_manager,
        reload,
        metrics,
        upgrade,
    } = params;
    let mut pingora_server_conf =
        ServerConf::new().expect("Could not construct pingora server configuration");

    pingora_server_conf.ca_file = config.server.ca_file.clone();
    pingora_server_conf.work_stealing = config.server.performance.work_stealing;
    pingora_server_conf.grace_period_seconds = config.server.shutdown.drain_seconds;
    pingora_server_conf.graceful_shutdown_timeout_seconds =
        config.server.shutdown.force_timeout_seconds;

    if let Some(sock) = &config.server.upgrade.sock {
        pingora_server_conf.upgrade_sock = sock.clone();
    }

    if let Some(retries) = config.server.upgrade.max_retries {
        pingora_server_conf.upgrade_sock_connect_accept_max_retries = Some(retries);
    }

    if let Some(threads) = config.server.threads {
        debug!(
            threads,
            "Creating Pingora server with overridden worker threads"
        );
        pingora_server_conf.threads = threads;
    }

    if let Some(pool_size) = config.server.upstream.connection_pool_size {
        pingora_server_conf.upstream_keepalive_pool_size = pool_size;
    }

    if let Some(accepts) = config.server.performance.parallel_accepts_per_listener {
        pingora_server_conf.listener_tasks_per_fd = accepts;
    }

    if let Some(source_addrs) = &config.server.upstream.source_addresses {
        pingora_server_conf.client_bind_to_ipv4 = source_addrs.ipv4.clone();
        pingora_server_conf.client_bind_to_ipv6 = source_addrs.ipv6.clone();
    }

    let opt = Opt {
        upgrade,
        ..Default::default()
    };
    let mut server = Server::new_with_opt_and_conf(Some(opt), pingora_server_conf);

    // In upgrade mode, signal the old process BEFORE bootstrap() blocks on
    // the upgrade socket. The old process's send_fds_to retries on
    // ENOENT/ECONNREFUSED, so it tolerates the socket not existing yet.
    if upgrade {
        crate::upgrade::signal_old_process(&config.server.pid_file)?;
    }

    server.bootstrap();

    //-------------------------------------------------------------------------
    // Public Proxy: Create public listener(s).
    //-------------------------------------------------------------------------
    // Global upstream timeouts.
    let upstream_connect_timeout = config.server.upstream.connection_timeout;
    let upstream_read_timeout = config.server.upstream.read_timeout;

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
            metrics.clone(),
            upstream_connect_timeout,
            upstream_read_timeout,
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

        if listener_cfg.enable_http2
            && let Some(h2_cfg) = &listener_cfg.http2
        {
            let mut options = default_h2_options();
            if let Some(v) = h2_cfg.max_concurrent_streams {
                options.max_concurrent_streams(v);
            }
            if let Some(v) = h2_cfg.max_header_list_size {
                options.max_header_list_size(v);
            }
            if let Some(v) = h2_cfg.initial_window_size {
                options.initial_window_size(v);
            }
            if let Some(v) = h2_cfg.initial_connection_window_size {
                options.initial_connection_window_size(v);
            }
            if let Some(app) = public_svc.app_logic_mut() {
                app.h2_options = Some(options);
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
            // Validation guarantees that every admin listener carries an
            // admin_auth block; treat its absence as a bug rather than
            // silently starting without authentication.
            let admin_auth = listener_cfg.admin_auth.clone().ok_or_else(|| {
                anyhow!(
                    "admin listener {} has no admin_auth configured. \
                     This is a bug: validation should have caught this.",
                    listener_cfg.name
                )
            })?;

            let admin_gateway = AdminGateway::new(
                traffic_manager.clone(),
                connection_manager.clone(),
                reload.clone(),
                cert_manager.clone(),
                Arc::new(admin_auth),
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
