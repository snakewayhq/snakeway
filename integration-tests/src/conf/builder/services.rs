use crate::conf::ConfigBuilder;
use snakeway_core::conf::types::{
    AcmeServerSpec, CertStoreSpec, EndpointSpec, EndpointTlsSpec, HostSpec, IngressSpec,
    ServiceRouteSpec, ServiceSpec, UpstreamSpec,
};
use std::path::PathBuf;

impl ConfigBuilder {
    pub fn with_grpc_ingress(mut self) -> Self {
        self.server_spec.ca_file = Some(PathBuf::from("./certs/ca.pem"));

        let mut bind = Self::make_bind(true);
        bind.enable_http2 = true;
        EndpointTlsSpec {
            sni: "".to_string(),
            verify: false,
            ca_file: Default::default(),
        };
        let service = ServiceSpec {
            routes: vec![ServiceRouteSpec {
                hosts: vec!["snakeway.test".to_string()],
                path: "/helloworld.Greeter/SayHello".to_string(),
                ..Default::default()
            }],
            upstreams: vec![
                Self::make_tcp_upstream(9000, true),
                Self::make_tcp_upstream(9001, true),
            ],
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
                hosts: vec!["snakeway.test".to_string()],
                path: "/ws".to_string(),
                enable_websocket: true,
                ..Default::default()
            }],
            upstreams: vec![
                Self::make_tcp_upstream(9000, false),
                Self::make_tcp_upstream(9001, false),
            ],
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

    pub fn with_https_ingress(mut self) -> Self {
        self.server_spec.ca_file = Some(PathBuf::from("./certs/ca.pem"));
        self.server_spec.tls_automation = Some(snakeway_core::conf::types::TlsAutomationSpec {
            acme: AcmeServerSpec {
                directory_url: "https://localhost:14000/dir".to_string(),
                data_dir: PathBuf::from("./certs/acme"),
                contact_email: vec!["barryallen@example.com".to_string()],
                ca_file: Some(PathBuf::from("./certs/pebble-ca.pem")),
            },
            cert_store: CertStoreSpec::Filesystem {
                cert_dir: PathBuf::from("./certs/acme"),
            },
            renew_within_days: 30,
        });
        let bind = Self::make_bind(true);
        let service = Self::make_service_spec();
        let ingress_spec = IngressSpec {
            bind: Some(bind),
            services: vec![service],
            ..Default::default()
        };
        self.ingress_specs.push(ingress_spec);
        self
    }

    pub(crate) fn make_tcp_upstream(port: u16, use_tls: bool) -> UpstreamSpec {
        UpstreamSpec {
            endpoint: Some(EndpointSpec {
                host: HostSpec::Hostname("snakeway.test".to_string()),
                port,
                tls: if use_tls {
                    Some(EndpointTlsSpec {
                        sni: "snakeway.test".to_string(),
                        verify: false,
                        ca_file: Default::default(),
                    })
                } else {
                    None
                },
            }),
            weight: 1,
            ..Default::default()
        }
    }

    pub(crate) fn make_service_spec() -> ServiceSpec {
        ServiceSpec {
            routes: vec![ServiceRouteSpec {
                hosts: vec!["snakeway.test".to_string()],
                path: "/api".to_string(),
                ..Default::default()
            }],
            upstreams: vec![
                Self::make_tcp_upstream(9000, false),
                Self::make_tcp_upstream(9001, false),
            ],
            ..Default::default()
        }
    }
}
