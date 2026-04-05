use crate::constants::{CERT_SERVER_KEY, CERT_SERVER_PEM, DEFAULT_LISTENER_PORT, TEST_HOST};
use snakeway_core::testing_api::conf::types::{
    AcmeChallengeSpec, BindInterfaceInput, BindSpec, ConnectionRateLimitingFilterSpec, DeviceSpec,
    IdentityDeviceSpec, IngressSpec, NetworkConnectionFilterSpec, NetworkPolicyDeviceSpec,
    RequestFilterDeviceSpec, RequestRateLimitingDeviceSpec, ServerSpec,
    StructuredLoggingDeviceSpec, TlsTerminationSpec,
};
use snakeway_core::testing_api::conf::{load_config_from_specs, types::RuntimeConfig};
use std::path::PathBuf;

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
                version: 1,
                threads: Some(1),
                ..Default::default()
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
            port: DEFAULT_LISTENER_PORT,
            tls: include_tls.then_some(TlsTerminationSpec::Manual {
                cert: PathBuf::from(CERT_SERVER_PEM),
                key: PathBuf::from(CERT_SERVER_KEY),
            }),
            ..Default::default()
        }
    }

    pub(crate) fn make_bind_with_acme() -> BindSpec {
        BindSpec {
            interface: BindInterfaceInput::Keyword("loopback".to_string()),
            port: DEFAULT_LISTENER_PORT,
            tls: Some(TlsTerminationSpec::Acme {
                domains: vec![TEST_HOST.to_string()],
                challenge: AcmeChallengeSpec::Http01,
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
            panic!("failed to load fixture config - check above for violations");
        }

        validated_cfg.config
    }
}

/// Network Connection Filter
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

    pub fn with_connection_filter(self, spec: NetworkConnectionFilterSpec) -> Self {
        self.set_connection_filter_on_last_bind(&spec)
    }
}

// Rate Limiter
impl ConfigBuilder {
    fn set_rate_limiter_on_last_bind(
        mut self,
        rate_limiter: &ConnectionRateLimitingFilterSpec,
    ) -> Self {
        self.ingress_specs
            .last_mut()
            .expect("no ingress specs found - cannot set connection filter")
            .bind
            .as_mut()
            .expect("no ingress specs found - cannot set connection filter")
            .connection_rate_limiting_filter = Some(rate_limiter.clone());
        self
    }

    pub fn with_connection_rate_limiting_filter(
        self,
        max_connections_per_second: u16,
        window_seconds: u16,
    ) -> Self {
        let rate_limiter = ConnectionRateLimitingFilterSpec {
            max_connections_per_second,
            window_seconds,
        };
        self.set_rate_limiter_on_last_bind(&rate_limiter)
    }
}
