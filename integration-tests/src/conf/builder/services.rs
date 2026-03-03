use crate::conf::ConfigBuilder;
use snakeway_core::conf::types::{
    AcmeServerSpec, BindAdminSpec, BindInterfaceInput, CertStoreSpec, EndpointSpec,
    EndpointTlsSpec, HostSpec, IngressSpec, RedirectSpec, ServiceRouteSpec, ServiceSpec,
    TlsTerminationSpec, UpstreamSpec,
};
use std::path::PathBuf;

impl ConfigBuilder {
    pub fn with_grpc_ingress(mut self) -> Self {
        self.server_spec.ca_file = Some(PathBuf::from("./certs/origin-ca.pem"));

        let mut bind = Self::make_bind(true);
        bind.enable_http2 = true;
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
        self.server_spec.ca_file = Some(PathBuf::from("./certs/origin-ca.pem"));
        self.server_spec.tls_automation = Some(snakeway_core::conf::types::TlsAutomationSpec {
            acme: AcmeServerSpec {
                directory_url: "https://localhost:14000/dir".to_string(),
                data_dir: PathBuf::from("./acme/orders/"),
                contact_email: vec!["barryallen@example.com".to_string()],
                ca_file: Some(PathBuf::from("./certs/pebble-ca.pem")),
            },
            // Memory store avoids filesystem path concerns in tests.
            cert_store: CertStoreSpec::Filesystem {
                cert_dir: PathBuf::from("./acme/certs/"),
            },
            renew_within_days: 30,
        });

        // Public HTTPS listener.  Port 5002 is Pebble's httpPort (see pebble.json):
        // the redirect listener on that port answers HTTP-01 challenges during ACME issuance.
        let mut bind = Self::make_bind_with_acme();
        bind.redirect_http_to_https = Some(RedirectSpec {
            port: 5002,
            status: 301,
        });
        let service = Self::make_service_spec();
        let ingress_spec = IngressSpec {
            bind: Some(bind),
            services: vec![service],
            ..Default::default()
        };
        self.ingress_specs.push(ingress_spec);

        // Admin API listener (manual TLS with the test server cert).
        let admin_ingress = IngressSpec {
            bind_admin: Some(BindAdminSpec {
                interface: BindInterfaceInput::Keyword("loopback".to_string()),
                port: 9443,
                tls: TlsTerminationSpec::Manual {
                    cert: PathBuf::from("./certs/server.pem"),
                    key: PathBuf::from("./certs/server.key"),
                },
                ..Default::default()
            }),
            ..Default::default()
        };
        self.ingress_specs.push(admin_ingress);

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
