use crate::conf::merge::{merge_listeners, merge_services};
use crate::conf::types::{
    IngressConfig, ListenerConfig, RedirectConfig, RouteConfig, ServiceConfig, ServiceRouteConfig,
    StaticCachePolicy, StaticFileConfig, StaticRouteConfig, UpstreamTcpConfig, UpstreamUnixConfig,
};
use crate::conf::validation::ConfigError;
use std::collections::HashMap;

pub type IrConfig = (
    Vec<ListenerConfig>,
    Vec<RouteConfig>,
    HashMap<String, ServiceConfig>,
);

/// Transform DSL configs to intermediate representation.
pub fn lower_configs(ingresses: Vec<IngressConfig>) -> Result<IrConfig, ConfigError> {
    let mut listeners: Vec<ListenerConfig> = Vec::new();
    let mut routes: Vec<RouteConfig> = Vec::new();
    let mut services: Vec<ServiceConfig> = Vec::new();

    for (idx, ingress) in ingresses.into_iter().enumerate() {
        let listener_name = format!("listener-{}", idx);

        for redirect_cfg in ingress.redirect_cfgs {
            listeners.push(ListenerConfig {
                name: listener_name.clone(),
                addr: redirect_cfg.addr,
                tls: None,
                enable_http2: false,
                enable_admin: false,
                redirect: Some(RedirectConfig {
                    to: redirect_cfg.to,
                    status: redirect_cfg.status,
                }),
            });
        }

        if let Some(bind_admin) = ingress.bind_admin {
            listeners.push(ListenerConfig {
                name: listener_name.clone(),
                addr: bind_admin.addr,
                tls: Some(bind_admin.tls),
                enable_http2: false,
                enable_admin: true,
                redirect: None,
            });
        }

        if let Some(bind) = ingress.bind {
            let use_tls = bind.tls.is_some();

            //-----------------------------------------------------------------
            // Services
            //-----------------------------------------------------------------
            for service_cfg in ingress.service_cfgs {
                let unix_upstreams = service_cfg
                    .backends
                    .iter()
                    .filter_map(|b| {
                        b.unix.as_ref().map(|unix| UpstreamUnixConfig {
                            weight: b.weight,
                            sock: unix.sock.clone(),
                            use_tls,
                            sni: "localhost".to_string(),
                        })
                    })
                    .collect();

                let tcp_upstreams = service_cfg
                    .backends
                    .iter()
                    .filter_map(|b| {
                        b.tcp.as_ref().map(|tcp| UpstreamTcpConfig {
                            weight: b.weight,
                            url: match use_tls {
                                true => format!("https://{}", tcp.addr),
                                false => format!("http://{}", tcp.addr),
                            },
                        })
                    })
                    .collect();

                let service_name = format!("{}-service", bind.addr.clone());

                services.push(ServiceConfig {
                    name: service_name.clone(),
                    listener: listener_name.clone(),
                    strategy: service_cfg.strategy,
                    tcp_upstreams,
                    unix_upstreams,
                    circuit_breaker: service_cfg.circuit_breaker.unwrap_or_default(),
                    health_check: service_cfg.health_check.unwrap_or_default(),
                });

                for route in service_cfg.routes {
                    routes.push(RouteConfig::Service(ServiceRouteConfig {
                        path: route.path,
                        listener: listener_name.clone(),
                        service: service_name.clone(),
                        allow_websocket: route.enable_websocket,
                        ws_max_connections: route.ws_max_connections,
                    }));
                }
            }

            //-----------------------------------------------------------------
            // Static files
            //-----------------------------------------------------------------
            for static_cfg in ingress.static_cfgs {
                for route in static_cfg.routes {
                    routes.push(RouteConfig::Static(StaticRouteConfig {
                        path: route.path,
                        file_dir: route.file_dir,
                        index: route.index.clone(),
                        directory_listing: route.directory_listing,
                        static_config: StaticFileConfig {
                            max_file_size: route.max_file_size,
                            small_file_threshold: route.compression.small_file_threshold,
                            min_gzip_size: route.compression.min_gzip_size,
                            min_brotli_size: route.compression.min_brotli_size,
                            enable_gzip: route.compression.enable_gzip,
                            enable_brotli: route.compression.enable_brotli,
                        },
                        cache_policy: StaticCachePolicy {
                            max_age_secs: route.cache_policy.max_age_secs,
                            public: route.cache_policy.public,
                            immutable: route.cache_policy.immutable,
                        },
                        listener: listener_name.clone(),
                    }));
                }
            }

            listeners.push(ListenerConfig {
                name: listener_name.clone(),
                addr: bind.addr,
                tls: bind.tls,
                enable_http2: false,
                enable_admin: false,
                redirect: None,
            });
        }
    }

    let (merged_listeners, name_map) = merge_listeners(listeners)?;

    for route in &mut routes {
        route.set_listener(name_map[route.listener()].clone());
    }

    for service in services.iter_mut() {
        service.listener = name_map[&service.listener].clone();
    }

    // Services have to be merged after rewriting listener names
    let merged_services: HashMap<String, ServiceConfig> = merge_services(services.clone())?;

    Ok((merged_listeners, routes, merged_services))
}
