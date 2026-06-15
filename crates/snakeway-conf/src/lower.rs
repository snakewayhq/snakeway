use crate::types::{
    DeviceConfig, DeviceSpec, IdentityDeviceConfig, IngressSpec, ListenerConfig,
    NetworkPolicyDeviceConfig, RequestFilterDeviceConfig, RequestRateLimitingDeviceConfig,
    RouteConfig, RuntimeConfig, ServerConfig, ServiceConfig, ServiceRouteConfig, StaticRouteConfig,
    StructuredLoggingDeviceConfig, UpstreamTcpConfig, UpstreamUnixConfig, WasmDeviceConfig,
};
use confval::provenance::{Located, Lower, Report, Validate, narrow};
use std::collections::HashMap;
use std::net::SocketAddr;

/// Transform spec to the runtime configuration.
///
/// Assumes all specs have already passed validation, so any error reported
/// here indicates a missing validation rule. Lowering continues across
/// ingresses and devices so every such error is reported in one pass;
/// `None` is returned when any step failed.
///
/// The `where IngressSpec: Validate` bound is the ingress-family equivalent of
/// the `Lower` bound the server and device configs carry: the ingress lowering
/// is a flattening rather than a per-entity `Lower` impl, so the bound lives
/// here, on the function that performs it. Through `IngressSpec`'s
/// compositional `Validate` impl it transitively requires every ingress child
/// entity to be validatable, enforced at compile time.
pub(crate) fn lower_configs(
    server: ServerConfig,
    ingresses: Vec<Located<IngressSpec>>,
    device_specs: Vec<Located<DeviceSpec>>,
    report: &mut Report,
) -> Option<RuntimeConfig>
where
    IngressSpec: Validate,
{
    let mut listeners = Vec::new();
    let mut routes = Vec::new();
    let mut services = HashMap::new();
    let mut failed = false;

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
            match ListenerConfig::from_bind_admin(&listener_name, &bind_admin.value, report) {
                Some(listener_cfg) => listeners.push(listener_cfg),
                None => failed = true,
            }
        }

        //--------------------------------------------------------------------
        // Public bind
        //--------------------------------------------------------------------
        if let Some(bind) = &ingress.bind {
            let bind = &bind.value;
            let use_tls = bind.tls.is_some();
            // The address names the services below, so nothing else in this
            // bind can lower without it.
            let bind_addr = match bind.resolve() {
                Ok(addr) => addr,
                Err(e) => {
                    report
                        .error(format!("invalid bind address: {e}"))
                        .at(bind.interface.span)
                        .emit();
                    failed = true;
                    continue;
                }
            };

            //-----------------------------------------------------------------
            // Services
            //-----------------------------------------------------------------
            for service_spec in &ingress.services {
                let service_spec = &service_spec.value;
                // Weight narrows through `narrow::` so a negative or oversized
                // value is reported and rejected rather than wrapping to u32.
                let mut unix_upstreams = Vec::new();
                let mut tcp_upstreams = Vec::new();
                for u in &service_spec.upstreams {
                    let Some(weight) = narrow::i64_to_u32(&u.value.weight, report) else {
                        failed = true;
                        continue;
                    };
                    if let Some(sock) = &u.value.sock {
                        unix_upstreams.push(UpstreamUnixConfig::new(
                            sock.value.clone(),
                            use_tls,
                            weight,
                        ));
                    }
                    if let Some(endpoint) = &u.value.endpoint {
                        tcp_upstreams.push(UpstreamTcpConfig::new(weight, &endpoint.value));
                    }
                }

                let service_name = format!("{}-service", bind_addr);

                let service = ServiceConfig::new(
                    &service_name,
                    &listener_name,
                    tcp_upstreams,
                    unix_upstreams,
                    service_spec,
                    report,
                );
                let Some(service) = service else {
                    failed = true;
                    continue;
                };

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
                    match StaticRouteConfig::new(&listener_name, &route.value, report) {
                        Some(cfg) => routes.push(RouteConfig::Static(cfg)),
                        None => failed = true,
                    }
                }
            }

            //-----------------------------------------------------------------
            // Listener
            //-----------------------------------------------------------------
            match ListenerConfig::from_bind(&listener_name, bind, report) {
                Some(listener_cfg) => listeners.push(listener_cfg),
                None => failed = true,
            }

            //-----------------------------------------------------------------
            // Redirect listener
            //-----------------------------------------------------------------
            if let Some(redirect) = &bind.redirect_http_to_https {
                let redirect_listener_name = format!("redirect-listener-{}", idx);

                let mut socket: SocketAddr = bind_addr;
                socket.set_port(redirect.value.port.value as u16);

                match ListenerConfig::from_redirect(
                    &redirect_listener_name,
                    socket.to_string(),
                    redirect.value.status.value as u16,
                    bind,
                    report,
                ) {
                    Some(listener_cfg) => listeners.push(listener_cfg),
                    None => failed = true,
                }
            }
        }
    }

    //-------------------------------------------------------------------------
    // Devices
    //-------------------------------------------------------------------------
    let mut devices = Vec::with_capacity(device_specs.len());
    for spec in device_specs {
        let lowered = match spec.value {
            DeviceSpec::RequestFilter(d) => RequestFilterDeviceConfig::lower(&d, report)
                .map(|c| DeviceConfig::RequestFilter(Box::new(c))),
            DeviceSpec::Identity(d) => {
                IdentityDeviceConfig::lower(&d, report).map(DeviceConfig::Identity)
            }
            DeviceSpec::NetworkPolicy(d) => {
                NetworkPolicyDeviceConfig::lower(&d, report).map(DeviceConfig::NetworkPolicy)
            }
            DeviceSpec::Wasm(d) => WasmDeviceConfig::lower(&d, report).map(DeviceConfig::Wasm),
            DeviceSpec::StructuredLogging(d) => StructuredLoggingDeviceConfig::lower(&d, report)
                .map(DeviceConfig::StructuredLogging),
            DeviceSpec::RequestRateLimiting(d) => {
                RequestRateLimitingDeviceConfig::lower(&d, report)
                    .map(DeviceConfig::RequestRateLimiting)
            }
        };
        match lowered {
            Some(device) => devices.push(device),
            None => failed = true,
        }
    }

    if failed {
        debug_assert!(report.has_errors());
        return None;
    }

    Some(RuntimeConfig {
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
        let result = lower_configs(server, vec![ingress], vec![], &mut Report::new());

        // Assert
        let config = result.expect("lowering should succeed");
        assert_eq!(config.listeners.len(), 1);
        assert_eq!(config.services.len(), 1);
        assert!(!config.routes.is_empty());
    }

    #[test]
    fn negative_upstream_weight_is_rejected_not_wrapped() {
        // Arrange: a negative weight must not lower to a wrapped u32.
        let server = lowered_server();
        let ingress = Located::detached(IngressSpec {
            bind: Some(Located::detached(BindSpec {
                interface: Located::detached("loopback".to_string()),
                port: Located::detached(8080),
                ..Default::default()
            })),
            services: vec![Located::detached(ServiceSpec {
                load_balancing_strategy: Located::detached("failover".to_string()),
                upstreams: vec![Located::detached(UpstreamSpec {
                    endpoint: Some(Located::detached(EndpointSpec {
                        host: Located::detached("127.0.0.1".to_string()),
                        port: Located::detached(3000),
                        tls: None,
                    })),
                    sock: None,
                    weight: Located::detached(-1),
                })],
                ..Default::default()
            })],
            ..Default::default()
        });
        let mut report = Report::new();

        // Act
        let result = lower_configs(server, vec![ingress], vec![], &mut report);

        // Assert
        assert!(result.is_none());
        assert!(report.has_errors());
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
        let result = lower_configs(server, vec![ingress], vec![], &mut Report::new());

        // Assert
        let config = result.expect("lowering should succeed");
        assert_eq!(config.listeners.len(), 1);
    }

    #[test]
    fn lower_empty_ingresses() {
        // Arrange
        let server = lowered_server();

        // Act
        let result = lower_configs(server, vec![], vec![], &mut Report::new());

        // Assert
        let config = result.expect("lowering should succeed");
        assert!(config.listeners.is_empty());
        assert!(config.services.is_empty());
        assert!(config.routes.is_empty());
    }
}
