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
pub fn lower_configs(
    server_spec: ServerSpec,
    ingresses: Vec<IngressSpec>,
    device_specs: Vec<DeviceSpec>,
) -> Result<IrConfig, ConfigError> {
    let server: ServerConfig = ServerConfig {
        version: server_spec.version,
        threads: server_spec.threads,
        pid_file: server_spec.pid_file.unwrap_or_default(),
        ca_file: server_spec.ca_file.unwrap_or_default(),
    };

    let mut listeners: Vec<ListenerConfig> = Vec::new();
    let mut routes: Vec<RouteConfig> = Vec::new();
    let mut services: HashMap<String, ServiceConfig> = HashMap::new();

    for (idx, ingress) in ingresses.into_iter().enumerate() {
        let listener_name = format!("listener-{}", idx);

        if let Some(bind_admin) = ingress.bind_admin {
            let listener = ListenerConfig::from_bind_admin(&listener_name, bind_admin);
            listeners.push(listener);
        }

        if let Some(bind) = ingress.bind {
            let use_tls = bind.tls.is_some();

            //-----------------------------------------------------------------
            // Services
            //-----------------------------------------------------------------
            for service_cfg in ingress.service_cfgs {
                let unix_upstreams = service_cfg
                    .upstreams
                    .iter()
                    .filter_map(|u| {
                        u.sock
                            .as_ref()
                            .map(|sock| UpstreamUnixConfig::new(sock.clone(), use_tls, u.weight))
                    })
                    .collect();

                let tcp_upstreams = service_cfg
                    .upstreams
                    .iter()
                    .filter_map(|u| {
                        u.addr
                            .as_ref()
                            .map(|addr| UpstreamTcpConfig::new(addr, use_tls, u.weight))
                    })
                    .collect();

                let service_name = format!("{}-service", bind.addr.clone());

                let service = ServiceConfig::new(
                    &service_name,
                    &listener_name,
                    tcp_upstreams,
                    unix_upstreams,
                    &service_cfg,
                );
                services.insert(service_name.clone(), service);

                for route in service_cfg.routes {
                    let service_route =
                        ServiceRouteConfig::new(&service_name, &listener_name, route);
                    routes.push(RouteConfig::Service(service_route));
                }
            }

            //-----------------------------------------------------------------
            // Static files
            //-----------------------------------------------------------------
            for static_cfg in ingress.static_cfgs {
                for route in static_cfg.routes {
                    let static_route = StaticRouteConfig::new(&listener_name, route);
                    routes.push(RouteConfig::Static(static_route));
                }
            }

            //-----------------------------------------------------------------
            // Bind/Listener
            //-----------------------------------------------------------------
            listeners.push(ListenerConfig::from_bind(&listener_name, bind.clone()));

            //-----------------------------------------------------------------
            // Add listener
            //-----------------------------------------------------------------
            if let Some(redirect) = bind.clone().redirect_http_to_https {
                let listener_name = format!("redirect-listener-{}", idx);
                let mut socket: SocketAddr = bind.addr.parse().expect("invalid bind address");
                socket.set_port(redirect.port);
                let listener = ListenerConfig::from_redirect(
                    &listener_name,
                    socket.to_string(),
                    redirect.status,
                    bind,
                );
                listeners.push(listener);
            }
        }
    }

    //-------------------------------------------------------------------------
    // Devices
    //-------------------------------------------------------------------------
    let mut devices: Vec<DeviceConfig> = Vec::new();
    for device_spec in device_specs {
        let device_config = match device_spec {
            DeviceSpec::Wasm(d) => DeviceConfig::Wasm(d.into()),
            DeviceSpec::Identity(d) => DeviceConfig::Identity(d.into()),
            DeviceSpec::StructuredLogging(d) => DeviceConfig::StructuredLogging(d.into()),
        };
        devices.push(device_config);
    }

    Ok((server, listeners, routes, services, devices))
}
