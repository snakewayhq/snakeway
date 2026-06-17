use crate::conf::ConfigBuilder;
use crate::constants::{
    ACME_CERTS_DIR, ACME_CONTACT_EMAIL, ACME_DIRECTORY_URL, ACME_ORDERS_DIR, ADMIN_TOKEN_FILE,
    CERT_ORIGIN_CA_PEM, CERT_PEBBLE_CA_PEM, CERT_SERVER_KEY, CERT_SERVER_PEM,
    DEFAULT_ADMIN_LISTENER_PORT, ROUTE_PATH_API, ROUTE_PATH_GRPC, ROUTE_PATH_WS, TEST_HOST,
    UPSTREAM_PORT_PRIMARY, UPSTREAM_PORT_SECONDARY,
};
use confval::source::Located;
use snakeway_core::testing_api::conf::types::{
    AcmeServerSpec, AdminAuthSpec, BearerAuthSpec, BindAdminSpec, CertStoreSpec, EndpointSpec,
    EndpointTlsSpec, IngressSpec, RedirectSpec, ServiceRouteSpec, ServiceSpec, TlsTerminationSpec,
    UpstreamSpec,
};
use std::path::PathBuf;

fn test_admin_auth_spec() -> Option<Located<AdminAuthSpec>> {
    Some(Located::detached(AdminAuthSpec {
        bearer: Some(Located::detached(BearerAuthSpec {
            token_file: Located::detached(PathBuf::from(ADMIN_TOKEN_FILE)),
        })),
    }))
}

impl ConfigBuilder {
    pub fn with_grpc_ingress(mut self) -> Self {
        self.server_spec.ca_file = Some(Located::detached(PathBuf::from(CERT_ORIGIN_CA_PEM)));

        let mut bind = Self::make_bind(true);
        bind.enable_http2 = Located::detached(true);
        let service = ServiceSpec {
            routes: vec![Located::detached(ServiceRouteSpec {
                hosts: vec![Located::detached(TEST_HOST.to_string())],
                path: Located::detached(ROUTE_PATH_GRPC.to_string()),
                ..Default::default()
            })],
            upstreams: vec![
                Located::detached(Self::make_tcp_upstream(UPSTREAM_PORT_PRIMARY, true)),
                Located::detached(Self::make_tcp_upstream(UPSTREAM_PORT_SECONDARY, true)),
            ],
            ..Default::default()
        };
        let ingress_spec = IngressSpec {
            bind: Some(Located::detached(bind)),
            services: vec![Located::detached(service)],
            ..Default::default()
        };
        self.ingress_specs.push(ingress_spec);
        self
    }

    pub fn with_h2_to_h1_ingress(mut self) -> Self {
        let mut bind = Self::make_bind(true);
        bind.enable_http2 = Located::detached(true);
        let service = Self::make_service_spec();
        let ingress_spec = IngressSpec {
            bind: Some(Located::detached(bind)),
            services: vec![Located::detached(service)],
            ..Default::default()
        };
        self.ingress_specs.push(ingress_spec);
        self
    }

    pub fn with_ws_ingress(mut self) -> Self {
        let bind = Self::make_bind(false);
        let service = ServiceSpec {
            routes: vec![Located::detached(ServiceRouteSpec {
                hosts: vec![Located::detached(TEST_HOST.to_string())],
                path: Located::detached(ROUTE_PATH_WS.to_string()),
                enable_websocket: Located::detached(true),
                ..Default::default()
            })],
            upstreams: vec![
                Located::detached(Self::make_tcp_upstream(UPSTREAM_PORT_PRIMARY, false)),
                Located::detached(Self::make_tcp_upstream(UPSTREAM_PORT_SECONDARY, false)),
            ],
            ..Default::default()
        };
        let ingress_spec = IngressSpec {
            bind: Some(Located::detached(bind)),
            services: vec![Located::detached(service)],
            ..Default::default()
        };
        self.ingress_specs.push(ingress_spec);
        self
    }

    pub fn with_custom_ingress(mut self, services: Vec<ServiceSpec>) -> Self {
        let bind = Self::make_bind(false);
        let ingress_spec = IngressSpec {
            bind: Some(Located::detached(bind)),
            services: services.into_iter().map(Located::detached).collect(),
            ..Default::default()
        };
        self.ingress_specs.push(ingress_spec);
        self
    }

    pub fn with_http_ingress(mut self) -> Self {
        let bind = Self::make_bind(false);
        let service = Self::make_service_spec();
        let ingress_spec = IngressSpec {
            bind: Some(Located::detached(bind)),
            services: vec![Located::detached(service)],
            ..Default::default()
        };
        self.ingress_specs.push(ingress_spec);
        self
    }

    pub fn with_https_ingress(mut self) -> Self {
        // Pre-create the ACME directories before validation runs.  Validation
        // requires these to exist as directories (they hold private key material
        // and should be intentionally provisioned by the operator/installer).
        std::fs::create_dir_all(ACME_ORDERS_DIR).expect("failed to create ACME orders directory");
        std::fs::create_dir_all(ACME_CERTS_DIR).expect("failed to create ACME certs directory");

        self.server_spec.ca_file = Some(Located::detached(PathBuf::from(CERT_ORIGIN_CA_PEM)));
        self.server_spec.tls_automation = Some(Located::detached(
            snakeway_core::testing_api::conf::types::TlsAutomationSpec {
                acme: Located::detached(AcmeServerSpec {
                    directory_url: Located::detached(ACME_DIRECTORY_URL.to_string()),
                    data_dir: Located::detached(PathBuf::from(ACME_ORDERS_DIR)),
                    contact_email: vec![Located::detached(ACME_CONTACT_EMAIL.to_string())],
                    ca_file: Some(Located::detached(PathBuf::from(CERT_PEBBLE_CA_PEM))),
                }),
                // Memory store avoids filesystem path concerns in tests.
                cert_store: Located::detached(CertStoreSpec::Filesystem {
                    cert_dir: Located::detached(PathBuf::from(ACME_CERTS_DIR)),
                }),
                renew_within_days: Located::detached(30),
            },
        ));

        // Public HTTPS listener.  Port 5002 is Pebble's httpPort (see pebble.json):
        // the redirect listener on that port answers HTTP-01 challenges during ACME issuance.
        let mut bind = Self::make_bind_with_acme();
        bind.redirect_http_to_https = Some(Located::detached(RedirectSpec {
            port: Located::detached(5002),
            status: Located::detached(301),
        }));
        let service = Self::make_service_spec();
        let ingress_spec = IngressSpec {
            bind: Some(Located::detached(bind)),
            services: vec![Located::detached(service)],
            ..Default::default()
        };
        self.ingress_specs.push(ingress_spec);

        // Admin API listener (manual TLS with the test server cert).
        let admin_ingress = IngressSpec {
            bind_admin: Some(Located::detached(BindAdminSpec {
                interface: Located::detached("loopback".to_string()),
                port: Located::detached(9443),
                tls: Located::detached(TlsTerminationSpec::Manual {
                    cert: Located::detached(PathBuf::from(CERT_SERVER_PEM)),
                    key: Located::detached(PathBuf::from(CERT_SERVER_KEY)),
                }),
                auth: test_admin_auth_spec(),
            })),
            ..Default::default()
        };
        self.ingress_specs.push(admin_ingress);

        self
    }

    /// Adds a standalone manual-TLS admin listener to the config.
    ///
    /// This lets admin API tests run without the full ACME / Pebble setup
    /// required by `with_https_ingress()`.  The listener uses the same test
    /// server certificate as other TLS listeners, so test clients must call
    /// `danger_accept_invalid_certs(true)`.
    ///
    /// The admin listener requires bearer-token auth. Tests should attach
    /// `Authorization: Bearer {ADMIN_TOKEN}` (see `constants.rs`) to every
    /// admin request.
    pub fn with_admin_ingress(mut self) -> Self {
        let admin_ingress = IngressSpec {
            bind_admin: Some(Located::detached(BindAdminSpec {
                interface: Located::detached("loopback".to_string()),
                port: Located::detached(DEFAULT_ADMIN_LISTENER_PORT),
                tls: Located::detached(TlsTerminationSpec::Manual {
                    cert: Located::detached(PathBuf::from(CERT_SERVER_PEM)),
                    key: Located::detached(PathBuf::from(CERT_SERVER_KEY)),
                }),
                auth: test_admin_auth_spec(),
            })),
            ..Default::default()
        };
        self.ingress_specs.push(admin_ingress);
        self
    }

    pub fn make_tcp_upstream(port: i64, use_tls: bool) -> UpstreamSpec {
        UpstreamSpec {
            endpoint: Some(Located::detached(EndpointSpec {
                host: Located::detached(TEST_HOST.to_string()),
                port: Located::detached(port),
                tls: if use_tls {
                    Some(Located::detached(EndpointTlsSpec {
                        sni: Located::detached(TEST_HOST.to_string()),
                        verify: Located::detached(false),
                        ca_file: Default::default(),
                    }))
                } else {
                    None
                },
            })),
            sock: None,
            weight: Located::detached(1),
        }
    }

    pub(crate) fn make_service_spec() -> ServiceSpec {
        ServiceSpec {
            routes: vec![Located::detached(ServiceRouteSpec {
                hosts: vec![Located::detached(TEST_HOST.to_string())],
                path: Located::detached(ROUTE_PATH_API.to_string()),
                ..Default::default()
            })],
            upstreams: vec![
                Located::detached(Self::make_tcp_upstream(UPSTREAM_PORT_PRIMARY, false)),
                Located::detached(Self::make_tcp_upstream(UPSTREAM_PORT_SECONDARY, false)),
            ],
            ..Default::default()
        }
    }
}
