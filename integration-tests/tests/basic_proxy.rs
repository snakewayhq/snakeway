use integration_tests::harness::TestServer;
use pretty_assertions::assert_eq;
use reqwest::StatusCode;
use snakeway_core::conf::types::{
    BindInterfaceInput, BindSpec, DeviceSpec, EndpointSpec, HostSpec, IngressSpec, ServerSpec,
    ServiceRouteSpec, ServiceSpec, UpstreamSpec,
};
use std::net::{IpAddr, Ipv4Addr};

#[test]
fn should_proxy_to_upstream() {
    let server_spec = ServerSpec {
        origin: Default::default(),
        version: 1,
        threads: Some(1),
        pid_file: None,
        ca_file: None,
    };
    let bind = BindSpec {
        origin: Default::default(),
        interface: BindInterfaceInput::Keyword("loopback".to_string()),
        port: 8080,
        tls: None,
        enable_http2: false,
        redirect_http_to_https: None,
        connection_filter: None,
    };
    let service = ServiceSpec {
        origin: Default::default(),
        load_balancing_strategy: Default::default(),
        routes: vec![ServiceRouteSpec {
            origin: Default::default(),
            path: "/api".to_string(),
            enable_websocket: false,
            ws_max_connections: None,
        }],
        upstreams: vec![
            UpstreamSpec {
                origin: Default::default(),
                endpoint: Some(EndpointSpec {
                    host: HostSpec::Ip(IpAddr::V4(Ipv4Addr::LOCALHOST)),
                    port: 9000,
                }),
                sock: None,
                weight: 1,
            },
            UpstreamSpec {
                origin: Default::default(),
                endpoint: Some(EndpointSpec {
                    host: HostSpec::Ip(IpAddr::V4(Ipv4Addr::LOCALHOST)),
                    port: 9001,
                }),
                sock: None,
                weight: 1,
            },
        ],
        health_check: None,
        circuit_breaker: None,
    };
    let ingress = IngressSpec {
        origin: Default::default(),
        bind: Some(bind),
        bind_admin: None,
        services: vec![service],
        static_files: vec![],
    };
    let ingress_specs = vec![ingress];
    let device_specs: Vec<DeviceSpec> = vec![];
    let srv = TestServer::start_http_upstream_with_config(server_spec, ingress_specs, device_specs);

    let res = srv.get("/api").send().expect("request failed");

    assert_eq!(res.status(), StatusCode::OK);
}

#[test]
fn should_load_config_files() {
    let srv = TestServer::start_with_http_upstream("basic");

    let res = srv.get("/api").send().expect("request failed");

    assert_eq!(res.status(), StatusCode::OK);
}
