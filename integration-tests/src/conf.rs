use snakeway_core::conf::types::{
    BindInterfaceInput, BindSpec, DeviceSpec, EndpointSpec, HostSpec, IdentityDeviceSpec,
    IngressSpec, ServerSpec, ServiceRouteSpec, ServiceSpec, TlsSpec, UpstreamSpec,
};
use snakeway_core::conf::{RuntimeConfig, load_config_from_specs};
use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;

pub fn minimal_http_runtime_config() -> RuntimeConfig {
    ConfigBuilder::default().with_http_ingress().build()
}

pub fn minimal_grpc_runtime_config() -> RuntimeConfig {
    ConfigBuilder::default().with_grpc_ingress().build()
}

pub struct ConfigBuilder {
    server_spec: ServerSpec,
    ingress_specs: Vec<IngressSpec>,
    identity_device_spec: Option<IdentityDeviceSpec>,
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self {
            server_spec: ServerSpec {
                origin: Default::default(),
                version: 1,
                threads: Some(1),
                pid_file: None,
                ca_file: None,
            },
            ingress_specs: vec![],
            identity_device_spec: Some(Self::make_identity_device()),
        }
    }
}

impl ConfigBuilder {
    pub fn with_server_spec(mut self, server_spec: ServerSpec) -> Self {
        self.server_spec = server_spec;
        self
    }

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

    pub fn with_http_ingress(mut self) -> Self {
        let bind = BindSpec {
            origin: Default::default(),
            interface: BindInterfaceInput::Keyword("loopback".to_string()),
            port: 8080,
            enable_http2: false,
            ..Default::default()
        };
        let service = ServiceSpec {
            routes: vec![ServiceRouteSpec {
                path: "/api".to_string(),
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

    fn make_bind(include_tls: bool) -> BindSpec {
        BindSpec {
            interface: BindInterfaceInput::Keyword("loopback".to_string()),
            port: 8080,
            tls: include_tls.then_some(TlsSpec {
                cert: "./certs/server.pem".to_string(),
                key: "./certs/server.key".to_string(),
            }),
            ..Default::default()
        }
    }

    fn make_tcp_upstream(port: u16) -> UpstreamSpec {
        UpstreamSpec {
            endpoint: Some(EndpointSpec {
                host: HostSpec::Ip(IpAddr::V4(Ipv4Addr::LOCALHOST)),
                port,
            }),
            weight: 1,
            ..Default::default()
        }
    }

    pub fn with_identity_device_and_no_geo(mut self) -> Self {
        let mut identity_device = Self::make_identity_device();
        identity_device.enable_geoip = false;
        self.identity_device_spec = Some(identity_device);
        self
    }

    pub fn with_identity_device_and_trusted_proxy(mut self) -> Self {
        let mut identity_device = Self::make_identity_device();
        identity_device.trusted_proxies = vec!["127.0.0.1/32".to_string()];
        self.identity_device_spec = Some(identity_device);
        self
    }

    fn make_identity_device() -> IdentityDeviceSpec {
        IdentityDeviceSpec {
            enable: true,
            trusted_proxies: vec![],
            enable_geoip: true,
            geoip_city_db: Some(PathBuf::from(
                "fixtures/geoip/dbip-country-lite-2025-12.mmdb",
            )),
            enable_user_agent: true,
            ..Default::default()
        }
    }

    pub fn build(self) -> RuntimeConfig {
        let mut device_specs = vec![];
        if let Some(identity_device_spec) = self.identity_device_spec {
            device_specs.push(DeviceSpec::Identity(identity_device_spec));
        }

        let validated_cfg =
            load_config_from_specs(self.server_spec, self.ingress_specs, device_specs)
                .expect("failed to load fixture config");

        if validated_cfg.validation_report.has_violations() {
            validated_cfg.validation_report.render_pretty();
            panic!("failed to load fixture config");
        }

        validated_cfg.config
    }
}
