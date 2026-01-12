use crate::conf::types::{
    DeviceConfig, DeviceSpec, IngressSpec, ListenerConfig, RouteConfig, ServerConfig, ServerSpec,
    ServiceConfig, ServiceRouteConfig, StaticCachePolicy, StaticFileConfig, StaticRouteConfig,
    UpstreamTcpConfig, UpstreamUnixConfig,
};
use crate::conf::validation::ConfigError;
use std::collections::HashMap;

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

        for redirect_cfg in ingress.redirect_cfgs {
            let listener = ListenerConfig::from_redirect(listener_name.clone(), redirect_cfg);
            listeners.push(listener);
        }

        if let Some(bind_admin) = ingress.bind_admin {
            let listener = ListenerConfig::from_bind_admin(listener_name.clone(), bind_admin);
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

                services.insert(
                    service_name.clone(),
                    ServiceConfig {
                        name: service_name.clone(),
                        listener: listener_name.clone(),
                        load_balancing_strategy: service_cfg.load_balancing_strategy,
                        tcp_upstreams,
                        unix_upstreams,
                        circuit_breaker: service_cfg.circuit_breaker.unwrap_or_default(),
                        health_check: service_cfg.health_check.unwrap_or_default(),
                    },
                );

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
                            max_age_seconds: route.cache_policy.max_age_seconds,
                            public: route.cache_policy.public,
                            immutable: route.cache_policy.immutable,
                        },
                        listener: listener_name.clone(),
                    }));
                }
            }

            listeners.push(ListenerConfig::from_bind(listener_name.clone(), bind));
        }
    }

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
