use snakeway_core::conf::types::{
    BindInterfaceInput, BindSpec, CidrSpec, ConnectionFilterSpec, DeviceSpec, IdentityDeviceSpec,
    IngressSpec, IpFamilySpec, OnNoPeerAddrSpec, RequestFilterDeviceSpec, ServerSpec,
    StructuredLoggingDeviceSpec, TlsSpec,
};
use snakeway_core::conf::{RuntimeConfig, load_config_from_specs};

pub struct ConfigBuilder {
    pub server_spec: ServerSpec,
    pub ingress_specs: Vec<IngressSpec>,
    pub identity_device_spec: Option<IdentityDeviceSpec>,
    pub structured_logging_device_spec: Option<StructuredLoggingDeviceSpec>,
    pub request_filter_device_spec: Option<RequestFilterDeviceSpec>,
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
            structured_logging_device_spec: Some(Self::make_structured_logging_device()),
            request_filter_device_spec: None,
        }
    }
}

impl ConfigBuilder {
    pub fn with_server_spec(mut self, server_spec: ServerSpec) -> Self {
        self.server_spec = server_spec;
        self
    }

    pub(crate) fn make_bind(include_tls: bool) -> BindSpec {
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

    pub(crate) fn make_bind_with_connection_filter(include_tls: bool) -> BindSpec {
        let mut bind = Self::make_bind(include_tls);
        bind.connection_filter = Some(ConnectionFilterSpec {
            cidr: CidrSpec {
                allow: vec![],
                deny: vec![],
            },
            ip_family: IpFamilySpec {
                ipv4: true,
                ipv6: true,
            },
            on_no_peer_addr: OnNoPeerAddrSpec::Deny,
        });
        bind
    }

    pub fn build(self) -> RuntimeConfig {
        let mut device_specs = vec![];
        if let Some(identity_device_spec) = self.identity_device_spec {
            device_specs.push(DeviceSpec::Identity(identity_device_spec));
        }

        if let Some(structured_logging_device_spec) = self.structured_logging_device_spec {
            device_specs.push(DeviceSpec::StructuredLogging(
                structured_logging_device_spec,
            ));
        }

        if let Some(request_filter_device_spec) = self.request_filter_device_spec {
            device_specs.push(DeviceSpec::RequestFilter(request_filter_device_spec));
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
