use crate::conf::merge::merge_services;
use crate::conf::types::{
    ExposeConfig, ListenerConfig, RouteConfig, ServiceConfig, ServiceRouteConfig,
    StaticRouteConfig, UpstreamTcpConfig, UpstreamUnixConfig,
};
use crate::conf::validation::ConfigError;
use std::collections::HashMap;

type ConfigIntermediateRepresentation = (
    Vec<ListenerConfig>,
    Vec<RouteConfig>,
    HashMap<String, ServiceConfig>,
);

pub fn lower_expose_configs(
    exposes: Vec<ExposeConfig>,
) -> Result<ConfigIntermediateRepresentation, ConfigError> {
    let mut listeners: Vec<ListenerConfig> = Vec::new();
    let mut routes: Vec<RouteConfig> = Vec::new();
    let mut services: Vec<ServiceConfig> = Vec::new();

    for (expose_idx, expose) in exposes.into_iter().enumerate() {
        let listener_name = format!("expose-{}", expose_idx);
        match expose {
            ExposeConfig::Admin(cfg) => {
                listeners.push(ListenerConfig {
                    name: listener_name.clone(),
                    addr: cfg.addr,
                    tls: Some(cfg.tls),
                    enable_http2: false,
                    enable_admin: cfg.enable_admin,
                });
            }
            ExposeConfig::Redirect(cfg) => {
                listeners.push(ListenerConfig {
                    name: listener_name.clone(),
                    addr: cfg.addr,
                    tls: None,
                    enable_http2: false,
                    enable_admin: false,
                });
            }
            ExposeConfig::Service(cfg) => {
                let service_name = format!("{}-service", cfg.addr);
                listeners.push(ListenerConfig {
                    name: listener_name.clone(),
                    addr: cfg.addr.clone(),
                    tls: cfg.tls,
                    enable_http2: cfg.enable_http2,
                    enable_admin: false,
                });
                let unix_upstreams = cfg
                    .backends
                    .iter()
                    .filter_map(|b| {
                        b.unix.as_ref().map(|unix| UpstreamUnixConfig {
                            weight: b.weight,
                            sock: unix.sock.clone(),
                            use_tls: false,
                            sni: "localhost".to_string(),
                        })
                    })
                    .collect();

                let tcp_upstreams = cfg
                    .backends
                    .iter()
                    .filter_map(|b| {
                        b.tcp.as_ref().map(|tcp| UpstreamTcpConfig {
                            weight: b.weight,
                            url: tcp.addr.clone(),
                        })
                    })
                    .collect();

                services.push(ServiceConfig {
                    name: service_name.clone(),
                    listener: listener_name.clone(),
                    strategy: cfg.strategy,
                    tcp_upstreams,
                    unix_upstreams,
                    circuit_breaker: cfg.circuit_breaker.unwrap_or_default(),
                    health_check: cfg.health_check.unwrap_or_default(),
                });

                for route in cfg.routes {
                    routes.push(RouteConfig::Service(ServiceRouteConfig {
                        path: route.path,
                        listener: listener_name.clone(),
                        service: service_name.clone(),
                        allow_websocket: route.enable_websocket,
                        ws_max_connections: route.ws_max_connections,
                    }));
                }
            }
            ExposeConfig::Static(cfg) => {
                listeners.push(ListenerConfig {
                    name: listener_name.clone(),
                    addr: cfg.addr.clone(),
                    tls: cfg.tls.clone(),
                    enable_http2: false,
                    enable_admin: false,
                });

                for route in cfg.routes {
                    routes.push(RouteConfig::Static(StaticRouteConfig {
                        path: route.path,
                        file_dir: route.file_dir,
                        index: route.index.clone(),
                        directory_listing: route.directory_listing,
                        static_config: route.static_config,
                        cache_policy: route.cache_policy,
                        listener: listener_name.clone(),
                    }));
                }
            }
        };
    }

    let service_map: HashMap<String, ServiceConfig> = merge_services(services)?;

    Ok((listeners, routes, service_map))
}
