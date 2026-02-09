use snakeway_core::conf::types::{
    BindInterfaceInput, BindSpec, CidrSpec, DeviceSpec, IdentityDeviceSpec, IngressSpec,
    IpFamilySpec, NetworkConnectionFilterSpec, NetworkPolicyDeviceSpec, OnNoPeerAddrSpec,
    RequestFilterDeviceSpec, RequestRateLimitingDeviceSpec, ServerSpec,
    StructuredLoggingDeviceSpec, TlsSpec,
};
use snakeway_core::conf::{RuntimeConfig, load_config_from_specs};

pub struct ConfigBuilder {
    pub server_spec: ServerSpec,
    pub ingress_specs: Vec<IngressSpec>,
    pub identity_device_spec: Option<IdentityDeviceSpec>,
    pub structured_logging_device_spec: Option<StructuredLoggingDeviceSpec>,
    pub request_filter_device_spec: Option<RequestFilterDeviceSpec>,
    pub network_policy_device_spec: Option<NetworkPolicyDeviceSpec>,
    pub request_rate_limiting_device_spec: Option<RequestRateLimitingDeviceSpec>,
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
            identity_device_spec: None,
            structured_logging_device_spec: None,
            request_filter_device_spec: None,
            network_policy_device_spec: None,
            request_rate_limiting_device_spec: None,
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

    pub fn build(self) -> RuntimeConfig {
        let mut device_specs = vec![];

        // Identity
        if let Some(identity_device_spec) = self.identity_device_spec {
            device_specs.push(DeviceSpec::Identity(identity_device_spec));
        }

        // Structured Logging
        if let Some(structured_logging_device_spec) = self.structured_logging_device_spec {
            device_specs.push(DeviceSpec::StructuredLogging(
                structured_logging_device_spec,
            ));
        }

        // Request Filter
        if let Some(request_filter_device_spec) = self.request_filter_device_spec {
            device_specs.push(DeviceSpec::RequestFilter(request_filter_device_spec));
        }

        // Network Policy
        if let Some(network_policy_device_spec) = self.network_policy_device_spec {
            device_specs.push(DeviceSpec::NetworkPolicy(network_policy_device_spec));
        }

        // Request Rate Limiting
        if let Some(request_rate_limiting_device_spec) = self.request_rate_limiting_device_spec {
            device_specs.push(DeviceSpec::RequestRateLimiting(
                request_rate_limiting_device_spec,
            ));
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

/// Connection Filter
impl ConfigBuilder {
    fn set_connection_filter_on_last_bind(
        mut self,
        connection_filter: &NetworkConnectionFilterSpec,
    ) -> Self {
        self.ingress_specs
            .last_mut()
            .expect("no ingress specs found - cannot set connection filter")
            .bind
            .as_mut()
            .expect("no ingress specs found - cannot set connection filter")
            .connection_filter = Some(connection_filter.clone());
        self
    }

    pub(crate) fn make_connection_filter(
        cidr_allow: Option<&[&str]>,
        cidr_deny: Option<&[&str]>,
        ipv4_enabled: bool,
        ipv6_enabled: bool,
        on_no_peer_addr: OnNoPeerAddrSpec,
    ) -> NetworkConnectionFilterSpec {
        NetworkConnectionFilterSpec {
            cidr: CidrSpec {
                allow: cidr_allow
                    .unwrap_or(&[])
                    .iter()
                    .map(|c| c.parse().expect("invalid CIDR in allowlist"))
                    .collect(),
                deny: cidr_deny
                    .unwrap_or(&[])
                    .iter()
                    .map(|c| c.parse().expect("invalid CIDR in denylist"))
                    .collect(),
            },
            ip_family: IpFamilySpec {
                ipv4: ipv4_enabled,
                ipv6: ipv6_enabled,
            },
            on_no_peer_addr,
        }
    }
    pub fn with_connection_filter_cidr_deny_list(self, cidr_deny: &[&str]) -> Self {
        let connection_filter =
            Self::make_connection_filter(None, Some(cidr_deny), true, true, OnNoPeerAddrSpec::Deny);
        self.set_connection_filter_on_last_bind(&connection_filter)
    }

    pub fn with_connection_filter_deny_when_no_ip(self) -> Self {
        let connection_filter =
            Self::make_connection_filter(None, None, true, true, OnNoPeerAddrSpec::Deny);
        self.set_connection_filter_on_last_bind(&connection_filter)
    }

    pub fn with_connection_filter_ipv4_disabled(self) -> Self {
        let connection_filter =
            Self::make_connection_filter(None, None, false, true, OnNoPeerAddrSpec::Deny);
        self.set_connection_filter_on_last_bind(&connection_filter)
    }
}
