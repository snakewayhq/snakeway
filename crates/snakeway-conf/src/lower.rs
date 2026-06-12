use crate::types::{
    DeviceConfig, DeviceSpec, IngressSpec, ListenerConfig, RouteConfig, RuntimeConfig,
    ServerConfig, ServiceConfig, ServiceRouteConfig, StaticRouteConfig, UpstreamTcpConfig,
    UpstreamUnixConfig,
};
use crate::validation::ConfigError;
use confval::provenance::Located;
use std::collections::HashMap;
use std::net::SocketAddr;

/// Transform spec to the runtime configuration.
///
/// Assumes all specs have already passed validation. The server entity is
/// lowered by the caller (span-first pipeline) and passed in ready-made.
pub(crate) fn lower_configs(
    server: ServerConfig,
    ingresses: Vec<Located<IngressSpec>>,
    device_specs: Vec<Located<DeviceSpec>>,
) -> Result<RuntimeConfig, ConfigError> {
    let mut listeners = Vec::new();
    let mut routes = Vec::new();
    let mut services = HashMap::new();

    // ---------------------------------------------------------------------
    // Ingresses
    // ---------------------------------------------------------------------
    for (idx, ingress) in ingresses.iter().enumerate() {
        let ingress = &ingress.value;
        let listener_name = format!("listener-{}", idx);

        // -------------------------------------------------------------
        // Admin bind
        // -------------------------------------------------------------
        if let Some(bind_admin) = &ingress.bind_admin {
            let listener_cfg =
                ListenerConfig::from_bind_admin(&listener_name, &bind_admin.value)
                    .map_err(|err| ConfigError::InvalidAdminBindConfig { message: err })?;
            listeners.push(listener_cfg);
        }

        //--------------------------------------------------------------------
        // Public bind
        //--------------------------------------------------------------------
        if let Some(bind) = &ingress.bind {
            let bind = &bind.value;
            let use_tls = bind.tls.is_some();
            let bind_addr = bind
                .resolve()
                .map_err(|e| ConfigError::InvalidBindAddress {
                    message: e.to_string(),
                })?;

            //-----------------------------------------------------------------
            // Services
            //-----------------------------------------------------------------
            for service_spec in &ingress.services {
                let service_spec = &service_spec.value;
                let unix_upstreams = service_spec
                    .upstreams
                    .iter()
                    .filter_map(|u| {
                        u.value.sock.as_ref().map(|sock| {
                            UpstreamUnixConfig::new(
                                sock.value.clone(),
                                use_tls,
                                u.value.weight.value as u32,
                            )
                        })
                    })
                    .collect::<Vec<_>>();

                let tcp_upstreams = service_spec
                    .upstreams
                    .iter()
                    .filter_map(|u| {
                        u.value.endpoint.as_ref().map(|endpoint| {
                            UpstreamTcpConfig::new(u.value.weight.value as u32, &endpoint.value)
                        })
                    })
                    .collect::<Vec<_>>();

                let service_name = format!("{}-service", bind_addr);

                let service = ServiceConfig::new(
                    &service_name,
                    &listener_name,
                    tcp_upstreams,
                    unix_upstreams,
                    service_spec,
                )
                .map_err(|message| ConfigError::Custom { message })?;

                services.insert(service_name.clone(), service);

                for route in &service_spec.routes {
                    routes.push(RouteConfig::Service(ServiceRouteConfig::new(
                        &service_name,
                        &listener_name,
                        &route.value,
                    )));
                }
            }

            //-----------------------------------------------------------------
            // Static files
            //-----------------------------------------------------------------
            for static_cfg in &ingress.static_files {
                for route in &static_cfg.value.routes {
                    routes.push(RouteConfig::Static(StaticRouteConfig::new(
                        &listener_name,
                        &route.value,
                    )));
                }
            }

            //-----------------------------------------------------------------
            // Listener
            //-----------------------------------------------------------------
            let listener_cfg = ListenerConfig::from_bind(&listener_name, bind)
                .map_err(|err| ConfigError::InvalidBindAddress { message: err })?;
            listeners.push(listener_cfg);

            //-----------------------------------------------------------------
            // Redirect listener
            //-----------------------------------------------------------------
            if let Some(redirect) = &bind.redirect_http_to_https {
                let redirect_listener_name = format!("redirect-listener-{}", idx);

                let mut socket: SocketAddr = bind_addr;
                socket.set_port(redirect.value.port.value as u16);

                let listener_cfg = ListenerConfig::from_redirect(
                    &redirect_listener_name,
                    socket.to_string(),
                    redirect.value.status.value as u16,
                    bind,
                )
                .map_err(|err| ConfigError::InvalidBindAddress { message: err })?;

                listeners.push(listener_cfg);
            }
        }
    }

    //-------------------------------------------------------------------------
    // Devices
    //-------------------------------------------------------------------------
    let devices = device_specs
        .into_iter()
        .map(|spec| match spec.value {
            DeviceSpec::RequestFilter(d) => d
                .try_into()
                .map(|c| DeviceConfig::RequestFilter(Box::new(c))),
            DeviceSpec::Identity(d) => d
                .try_into()
                .map(DeviceConfig::Identity)
                .map_err(|message| ConfigError::Custom { message }),
            DeviceSpec::NetworkPolicy(d) => d.try_into().map(DeviceConfig::NetworkPolicy),
            DeviceSpec::Wasm(d) => Ok(DeviceConfig::Wasm(d.into())),
            DeviceSpec::StructuredLogging(d) => d
                .try_into()
                .map(DeviceConfig::StructuredLogging)
                .map_err(|message| ConfigError::Custom { message }),
            DeviceSpec::RequestRateLimiting(d) => Ok(DeviceConfig::RequestRateLimiting(d.into())),
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(RuntimeConfig {
        server,
        listeners,
        routes,
        services,
        devices,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        AdminAuthSpec, BearerAuthSpec, BindAdminSpec, BindSpec, EndpointSpec, IngressSpec,
        ServerSpec, ServiceRouteSpec, ServiceSpec, UpstreamSpec,
    };
    use confval::provenance::{Lower, Report};

    fn lowered_server() -> ServerConfig {
        ServerConfig::lower(&ServerSpec::default(), &mut Report::new()).unwrap()
    }
    use std::io::Write;

    #[test]
    fn lower_minimal_valid_config() {
        // Arrange
        let server = lowered_server();
        let ingress = Located::detached(IngressSpec {
            bind: Some(Located::detached(BindSpec {
                interface: Located::detached("loopback".to_string()),
                port: Located::detached(8080),
                ..Default::default()
            })),
            services: vec![Located::detached(ServiceSpec {
                load_balancing_strategy: Located::detached("failover".to_string()),
                routes: vec![Located::detached(ServiceRouteSpec {
                    path: Located::detached("/".to_string()),
                    hosts: vec![Located::detached("example.com".to_string())],
                    ..Default::default()
                })],
                upstreams: vec![Located::detached(UpstreamSpec {
                    endpoint: Some(Located::detached(EndpointSpec {
                        host: Located::detached("127.0.0.1".to_string()),
                        port: Located::detached(3000),
                        tls: None,
                    })),
                    sock: None,
                    weight: Located::detached(1),
                })],
                ..Default::default()
            })],
            ..Default::default()
        });

        // Act
        let result = lower_configs(server, vec![ingress], vec![]);

        // Assert
        let config = result.expect("lowering should succeed");
        assert_eq!(config.listeners.len(), 1);
        assert_eq!(config.services.len(), 1);
        assert!(!config.routes.is_empty());
    }

    #[test]
    fn lower_ingress_with_admin_bind() {
        // Arrange
        let mut token_file = tempfile::NamedTempFile::new().expect("tempfile");
        token_file
            .write_all(b"a9f1c38de4b67029c5d1e97f4a0ebac12d3b8ffc84e1d27a05f6cb9e83d21a04\n")
            .unwrap();

        let server = lowered_server();
        let ingress = Located::detached(IngressSpec {
            bind_admin: Some(Located::detached(BindAdminSpec {
                interface: Located::detached("loopback".to_string()),
                port: Located::detached(9090),
                auth: Some(Located::detached(AdminAuthSpec {
                    bearer: Some(Located::detached(BearerAuthSpec {
                        token_file: Located::detached(token_file.path().to_path_buf()),
                    })),
                })),
                ..Default::default()
            })),
            ..Default::default()
        });

        // Act
        let result = lower_configs(server, vec![ingress], vec![]);

        // Assert
        let config = result.expect("lowering should succeed");
        assert_eq!(config.listeners.len(), 1);
    }

    #[test]
    fn lower_empty_ingresses() {
        // Arrange
        let server = lowered_server();

        // Act
        let result = lower_configs(server, vec![], vec![]);

        // Assert
        let config = result.expect("lowering should succeed");
        assert!(config.listeners.is_empty());
        assert!(config.services.is_empty());
        assert!(config.routes.is_empty());
    }
}
