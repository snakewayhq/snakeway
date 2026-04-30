use crate::conf::ConfigBuilder;
use crate::constants::{
    ACME_CERTS_DIR, ACME_CONTACT_EMAIL, ACME_DIRECTORY_URL, ACME_ORDERS_DIR, ADMIN_TOKEN_FILE,
    CERT_ORIGIN_CA_PEM, CERT_PEBBLE_CA_PEM, CERT_SERVER_KEY, CERT_SERVER_PEM,
    DEFAULT_ADMIN_LISTENER_PORT, ROUTE_PATH_API, ROUTE_PATH_GRPC, ROUTE_PATH_WS, TEST_HOST,
    UPSTREAM_PORT_PRIMARY, UPSTREAM_PORT_SECONDARY,
};
use snakeway_core::testing_api::conf::types::{
    AcmeServerSpec, AdminAuthSpec, BearerAuthSpec, BindAdminSpec, BindInterfaceInput,
    CertStoreSpec, EndpointSpec, EndpointTlsSpec, HostSpec, IngressSpec, RedirectSpec,
    ServiceRouteSpec, ServiceSpec, TlsTerminationSpec, UpstreamSpec,
};
use std::path::PathBuf;

fn test_admin_auth_spec() -> AdminAuthSpec {
    AdminAuthSpec {
        bearer: Some(BearerAuthSpec {
            token_file: PathBuf::from(ADMIN_TOKEN_FILE),
            origin: Default::default(),
        }),
        origin: Default::default(),
    }
}

impl ConfigBuilder {
    pub fn with_grpc_ingress(mut self) -> Self {
        self.server_spec.ca_file = Some(PathBuf::from(CERT_ORIGIN_CA_PEM));

        let mut bind = Self::make_bind(true);
        bind.enable_http2 = true;
        let service = ServiceSpec {
            routes: vec![ServiceRouteSpec {
                hosts: vec![TEST_HOST.to_string()],
                path: ROUTE_PATH_GRPC.to_string(),
                ..Default::default()
            }],
            upstreams: vec![
                Self::make_tcp_upstream(UPSTREAM_PORT_PRIMARY, true),
                Self::make_tcp_upstream(UPSTREAM_PORT_SECONDARY, true),
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
                hosts: vec![TEST_HOST.to_string()],
                path: ROUTE_PATH_WS.to_string(),
                enable_websocket: true,
                ..Default::default()
            }],
            upstreams: vec![
                Self::make_tcp_upstream(UPSTREAM_PORT_PRIMARY, false),
                Self::make_tcp_upstream(UPSTREAM_PORT_SECONDARY, false),
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

    pub fn with_custom_ingress(mut self, services: Vec<ServiceSpec>) -> Self {
        let bind = Self::make_bind(false);
        let ingress_spec = IngressSpec {
            bind: Some(bind),
            services,
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
        // Pre-create the ACME directories before validation runs.  Validation
        // requires these to exist as directories (they hold private key material
        // and should be intentionally provisioned by the operator/installer).
        std::fs::create_dir_all(ACME_ORDERS_DIR).expect("failed to create ACME orders directory");
        std::fs::create_dir_all(ACME_CERTS_DIR).expect("failed to create ACME certs directory");

        self.server_spec.ca_file = Some(PathBuf::from(CERT_ORIGIN_CA_PEM));
        self.server_spec.tls_automation =
            Some(snakeway_core::testing_api::conf::types::TlsAutomationSpec {
                acme: AcmeServerSpec {
                    directory_url: ACME_DIRECTORY_URL.to_string(),
                    data_dir: PathBuf::from(ACME_ORDERS_DIR),
                    contact_email: vec![ACME_CONTACT_EMAIL.to_string()],
                    ca_file: Some(PathBuf::from(CERT_PEBBLE_CA_PEM)),
                },
                // Memory store avoids filesystem path concerns in tests.
                cert_store: CertStoreSpec::Filesystem {
                    cert_dir: PathBuf::from(ACME_CERTS_DIR),
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
                    cert: PathBuf::from(CERT_SERVER_PEM),
                    key: PathBuf::from(CERT_SERVER_KEY),
                },
                auth: test_admin_auth_spec(),
                ..Default::default()
            }),
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
            bind_admin: Some(BindAdminSpec {
                interface: BindInterfaceInput::Keyword("loopback".to_string()),
                port: DEFAULT_ADMIN_LISTENER_PORT,
                tls: TlsTerminationSpec::Manual {
                    cert: PathBuf::from(CERT_SERVER_PEM),
                    key: PathBuf::from(CERT_SERVER_KEY),
                },
                auth: test_admin_auth_spec(),
                ..Default::default()
            }),
            ..Default::default()
        };
        self.ingress_specs.push(admin_ingress);
        self
    }

    pub fn make_tcp_upstream(port: u16, use_tls: bool) -> UpstreamSpec {
        UpstreamSpec {
            endpoint: Some(EndpointSpec {
                host: HostSpec::Hostname(TEST_HOST.to_string()),
                port,
                tls: if use_tls {
                    Some(EndpointTlsSpec {
                        sni: TEST_HOST.to_string(),
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
                hosts: vec![TEST_HOST.to_string()],
                path: ROUTE_PATH_API.to_string(),
                ..Default::default()
            }],
            upstreams: vec![
                Self::make_tcp_upstream(UPSTREAM_PORT_PRIMARY, false),
                Self::make_tcp_upstream(UPSTREAM_PORT_SECONDARY, false),
            ],
            ..Default::default()
        }
    }
}
