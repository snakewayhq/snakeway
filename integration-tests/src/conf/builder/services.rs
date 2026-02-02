use crate::conf::ConfigBuilder;
use snakeway_core::conf::types::{
    EndpointSpec, HostSpec, IngressSpec, ServiceRouteSpec, ServiceSpec, UpstreamSpec,
};
use std::net::{IpAddr, Ipv4Addr};

impl ConfigBuilder {
    pub fn with_grpc_ingress(mut self) -> Self {
        self.server_spec.ca_file = Some("./certs/ca.pem".to_string());
        let mut bind = Self::make_bind(true);
        bind.enable_http2 = true;
        let service = ServiceSpec {
            routes: vec![ServiceRouteSpec {
                path: "/helloworld.Greeter/SayHello".to_string(),
                ..Default::default()
            }],
            upstreams: vec![Self::make_tcp_upstream(9000), Self::make_tcp_upstream(9001)],
            ..Default::default()
        };
        let ingress_spec = IngressSpec {
            bind: Some(bind),
            services: vec![service],
            ..Default::default()
        };
        self.ingress_specs.push(ingress_spec);
        self
    }

    pub fn with_ws_ingress(mut self) -> Self {
        let bind = Self::make_bind(false);
        let service = ServiceSpec {
            routes: vec![ServiceRouteSpec {
                path: "/ws".to_string(),
                enable_websocket: true,
                ..Default::default()
            }],
            upstreams: vec![Self::make_tcp_upstream(9000), Self::make_tcp_upstream(9001)],
            ..Default::default()
        };
        let ingress_spec = IngressSpec {
            bind: Some(bind),
            services: vec![service],
            ..Default::default()
        };
        self.ingress_specs.push(ingress_spec);
        self
    }

    pub fn with_http_ingress(mut self) -> Self {
        let bind = Self::make_bind(false);
        let service = Self::make_service_spec();
        let ingress_spec = IngressSpec {
            bind: Some(bind),
            services: vec![service],
            ..Default::default()
        };
        self.ingress_specs.push(ingress_spec);
        self
    }

    pub(crate) fn make_tcp_upstream(port: u16) -> UpstreamSpec {
        UpstreamSpec {
            endpoint: Some(EndpointSpec {
                host: HostSpec::Ip(IpAddr::V4(Ipv4Addr::LOCALHOST)),
                port,
            }),
            weight: 1,
            ..Default::default()
        }
    }

    pub(crate) fn make_service_spec() -> ServiceSpec {
        ServiceSpec {
            routes: vec![ServiceRouteSpec {
                path: "/api".to_string(),
                ..Default::default()
            }],
            upstreams: vec![Self::make_tcp_upstream(9000), Self::make_tcp_upstream(9001)],
            ..Default::default()
        }
    }
}
