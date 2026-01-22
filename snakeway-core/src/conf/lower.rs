use crate::conf::types::{
    DeviceConfig, DeviceSpec, IngressSpec, ListenerConfig, RouteConfig, ServerConfig, ServerSpec,
    ServiceConfig, ServiceRouteConfig, StaticRouteConfig, UpstreamTcpConfig, UpstreamUnixConfig,
};
use crate::conf::validation::ConfigError;
use std::collections::HashMap;
use std::net::SocketAddr;

pub type IrConfig = (
    ServerConfig,
    Vec<ListenerConfig>,
    Vec<RouteConfig>,
    HashMap<String, ServiceConfig>,
    Vec<DeviceConfig>,
);

/// Transform spec to the runtime configuration.
///
/// Assumes all specs have already passed validation.
pub fn lower_configs(
    server_spec: ServerSpec,
    ingresses: Vec<IngressSpec>,
    device_specs: Vec<DeviceSpec>,
) -> Result<IrConfig, ConfigError> {
    // ---------------------------------------------------------------------
    // Server
    // ---------------------------------------------------------------------
    let server = ServerConfig {
        version: server_spec.version,
        threads: server_spec.threads,
        pid_file: server_spec.pid_file.unwrap_or_default(),
        ca_file: server_spec.ca_file.unwrap_or_default(),
    };

    let mut listeners = Vec::new();
    let mut routes = Vec::new();
    let mut services = HashMap::new();

    // ---------------------------------------------------------------------
    // Ingresses
    // ---------------------------------------------------------------------
    for (idx, ingress) in ingresses.into_iter().enumerate() {
        let listener_name = format!("listener-{}", idx);

        // -------------------------------------------------------------
        // Admin bind
        // -------------------------------------------------------------
        if let Some(bind_admin) = ingress.bind_admin {
            listeners.push(ListenerConfig::from_bind_admin(&listener_name, bind_admin));
        }

        //--------------------------------------------------------------------
        // Public bind
        //--------------------------------------------------------------------
        if let Some(bind) = ingress.bind {
            let use_tls = bind.tls.is_some();
            // safe - validated already
            let bind_addr = bind
                .resolve()
                .expect("bind.resolve() must not fail after validation");

            //-----------------------------------------------------------------
            // Services
            //-----------------------------------------------------------------
            for service_spec in ingress.services {
                let unix_upstreams = service_spec
                    .upstreams
                    .iter()
                    .filter_map(|u| {
                        u.sock
                            .as_ref()
                            .map(|sock| UpstreamUnixConfig::new(sock.clone(), use_tls, u.weight))
                    })
                    .collect::<Vec<_>>();

                let tcp_upstreams = service_spec
                    .upstreams
                    .iter()
                    .filter_map(|u| {
                        u.endpoint
                            .as_ref()
                            .map(|endpoint| UpstreamTcpConfig::new(use_tls, u.weight, endpoint))
                    })
                    .collect::<Result<Vec<_>, _>>()
                    .expect("upstream.resolve() must not fail");

                let service_name = format!("{}-service", bind_addr);

                let service = ServiceConfig::new(
                    &service_name,
                    &listener_name,
                    tcp_upstreams,
                    unix_upstreams,
                    &service_spec,
                );

                services.insert(service_name.clone(), service);

                for route in service_spec.routes {
                    routes.push(RouteConfig::Service(ServiceRouteConfig::new(
                        &service_name,
                        &listener_name,
                        route,
                    )));
                }
            }

            //-----------------------------------------------------------------
            // Static files
            //-----------------------------------------------------------------
            for static_cfg in ingress.static_files {
                for route in static_cfg.routes {
                    routes.push(RouteConfig::Static(StaticRouteConfig::new(
                        &listener_name,
                        route,
                    )));
                }
            }

            //-----------------------------------------------------------------
            // Listener
            //-----------------------------------------------------------------
            listeners.push(ListenerConfig::from_bind(&listener_name, bind.clone()));

            //-----------------------------------------------------------------
            // Redirect listener
            //-----------------------------------------------------------------
            if let Some(ref redirect) = bind.redirect_http_to_https {
                let redirect_listener_name = format!("redirect-listener-{}", idx);

                let mut socket: SocketAddr = bind_addr;
                socket.set_port(redirect.port);

                listeners.push(ListenerConfig::from_redirect(
                    &redirect_listener_name,
                    socket.to_string(),
                    redirect.status,
                    bind,
                ));
            }
        }
    }

    //-------------------------------------------------------------------------
    // Devices
    //-------------------------------------------------------------------------
    let devices = device_specs
        .into_iter()
        .map(|spec| match spec {
            DeviceSpec::Wasm(d) => Ok(DeviceConfig::Wasm(d.into())),
            DeviceSpec::Identity(d) => Ok(DeviceConfig::Identity(d.into())),
            DeviceSpec::RequestFilter(d) => d.try_into().map(DeviceConfig::RequestFilter),
            DeviceSpec::StructuredLogging(d) => Ok(DeviceConfig::StructuredLogging(d.into())),
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok((server, listeners, routes, services, devices))
}
