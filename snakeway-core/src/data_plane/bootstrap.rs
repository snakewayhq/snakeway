use crate::control_plane::acme::CertManager;
use crate::control_plane::reload::ReloadHandle;
use crate::control_plane::runtime::RuntimeState;
use crate::data_plane::proxy::{AdminGateway, PublicGateway, RedirectGateway};
use crate::data_plane::tls_handshake::{CertMode, build_tls_callbacks};
use crate::data_plane::ws_connection_management::WsConnectionManager;
use crate::execution::device::core::registry::DeviceRegistry;
use crate::execution::traffic::TrafficManager;
use crate::net::{ConnectionRateLimitingFilter, NetworkConnectionFilter};
use anyhow::{Error, Result, anyhow};
use arc_swap::ArcSwap;
use openssl::ssl::SslFiletype;
use pingora::listeners::tls::TlsSettings;
use pingora::prelude::*;
use pingora::server::Server;
use pingora::server::configuration::ServerConf;
use snakeway_conf::types::{RuntimeConfig, TlsTerminationConfig};
use std::sync::Arc;
use tracing::{debug, warn};

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
