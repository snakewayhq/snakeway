use crate::types::{
    DeviceConfig, DeviceSpec, IngressSpec, ListenerConfig, RouteConfig, RuntimeConfig,
    ServerConfig, ServerSpec, ServiceConfig, ServiceRouteConfig, StaticRouteConfig,
    UpstreamTcpConfig, UpstreamUnixConfig,
};
use crate::validation::ConfigError;
use std::collections::HashMap;
use std::net::SocketAddr;

/// Transform spec to the runtime configuration.
///
/// Assumes all specs have already passed validation.
pub(crate) fn lower_configs(
    server_spec: ServerSpec,
    ingresses: Vec<IngressSpec>,
    device_specs: Vec<DeviceSpec>,
) -> Result<RuntimeConfig, ConfigError> {
    // ---------------------------------------------------------------------
    // Server
    // ---------------------------------------------------------------------
    let server =
        ServerConfig::try_from(server_spec).map_err(|e| ConfigError::InvalidServerConfig {
            message: e.to_string(),
        })?;

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
            let listener_cfg = ListenerConfig::from_bind_admin(&listener_name, bind_admin)
                .map_err(|err| ConfigError::InvalidBindAddress { message: err })?;
            listeners.push(listener_cfg);
        }

        //--------------------------------------------------------------------
        // Public bind
        //--------------------------------------------------------------------
        if let Some(bind) = ingress.bind {
            let use_tls = bind.tls.is_some();
            let bind_addr = bind
                .resolve()
                .map_err(|e| ConfigError::InvalidBindAddress {
                    message: e.to_string(),
                })?;

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
                            .map(|endpoint| UpstreamTcpConfig::new(u.weight, endpoint))
                    })
                    .collect::<Vec<_>>();

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
            let listener_cfg = ListenerConfig::from_bind(&listener_name, bind.clone())
                .map_err(|err| ConfigError::InvalidBindAddress { message: err })?;
            listeners.push(listener_cfg);

            //-----------------------------------------------------------------
            // Redirect listener
            //-----------------------------------------------------------------
            if let Some(ref redirect) = bind.redirect_http_to_https {
                let redirect_listener_name = format!("redirect-listener-{}", idx);

                let mut socket: SocketAddr = bind_addr;
                socket.set_port(redirect.port);

                let listener_cfg = ListenerConfig::from_redirect(
                    &redirect_listener_name,
                    socket.to_string(),
                    redirect.status,
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
        .map(|spec| match spec {
            DeviceSpec::RequestFilter(d) => d.try_into().map(DeviceConfig::RequestFilter),
            DeviceSpec::Identity(d) => Ok(DeviceConfig::Identity(d.into())),
            DeviceSpec::NetworkPolicy(d) => d.try_into().map(DeviceConfig::NetworkPolicy),
            DeviceSpec::Wasm(d) => Ok(DeviceConfig::Wasm(d.into())),
            DeviceSpec::StructuredLogging(d) => Ok(DeviceConfig::StructuredLogging(d.into())),
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
        BindAdminSpec, BindInterfaceInput, BindSpec, EndpointSpec, HostSpec, IngressSpec,
        ServerSpec, ServiceRouteSpec, ServiceSpec, UpstreamSpec,
    };
    use std::net::Ipv4Addr;

    #[test]
    fn lower_minimal_valid_config() {
        // Arrange
        let server_spec = ServerSpec {
            version: 1,
            ..Default::default()
        };
        let ingress = IngressSpec {
            bind: Some(BindSpec {
                interface: BindInterfaceInput::Keyword("loopback".to_string()),
                port: 8080,
                ..Default::default()
            }),
            services: vec![ServiceSpec {
                routes: vec![ServiceRouteSpec {
                    path: "/".to_string(),
                    hosts: vec!["example.com".to_string()],
                    ..Default::default()
                }],
                upstreams: vec![UpstreamSpec {
                    endpoint: Some(EndpointSpec {
                        host: HostSpec::Ip(std::net::IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))),
                        port: 3000,
                        tls: None,
                    }),
                    weight: 1,
                    ..Default::default()
                }],
                ..Default::default()
            }],
            ..Default::default()
        };

        // Act
        let result = lower_configs(server_spec, vec![ingress], vec![]);

        // Assert
        let config = result.expect("lowering should succeed");
        assert_eq!(config.listeners.len(), 1);
        assert_eq!(config.services.len(), 1);
        assert!(!config.routes.is_empty());
    }

    #[test]
    fn lower_ingress_with_admin_bind() {
        // Arrange
        let server_spec = ServerSpec {
            version: 1,
            ..Default::default()
        };
        let ingress = IngressSpec {
            bind_admin: Some(BindAdminSpec {
                interface: BindInterfaceInput::Keyword("loopback".to_string()),
                port: 9090,
                ..Default::default()
            }),
            ..Default::default()
        };

        // Act
        let result = lower_configs(server_spec, vec![ingress], vec![]);

        // Assert
        let config = result.expect("lowering should succeed");
        assert_eq!(config.listeners.len(), 1);
    }

    #[test]
    fn lower_empty_ingresses() {
        // Arrange
        let server_spec = ServerSpec {
            version: 1,
            ..Default::default()
        };

        // Act
        let result = lower_configs(server_spec, vec![], vec![]);

        // Assert
        let config = result.expect("lowering should succeed");
        assert!(config.listeners.is_empty());
        assert!(config.services.is_empty());
        assert!(config.routes.is_empty());
    }
}
