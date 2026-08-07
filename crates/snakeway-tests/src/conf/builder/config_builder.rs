use crate::constants::{CERT_SERVER_KEY, CERT_SERVER_PEM, DEFAULT_LISTENER_PORT, TEST_HOST};
use confval::source::Located;
use snakeway::testing_api::conf::types::{
    AcmeChallenge, BindSpec, ConnectionRateLimitingFilterSpec, DeviceSpec, IdentityDeviceSpec,
    IngressSpec, NetworkConnectionFilterSpec, NetworkPolicyDeviceSpec, RequestFilterDeviceSpec,
    RequestRateLimitingDeviceSpec, ServerSpec, StructuredLoggingDeviceSpec, TlsTerminationSpec,
    WasmDeviceSpec,
};
use snakeway::testing_api::conf::{load_config_from_specs, types::RuntimeConfig};
use std::path::PathBuf;

pub struct ConfigBuilder {
    pub server_spec: ServerSpec,
    pub ingress_specs: Vec<IngressSpec>,
    pub identity_device_spec: Option<IdentityDeviceSpec>,
    pub structured_logging_device_spec: Option<StructuredLoggingDeviceSpec>,
    pub request_filter_device_spec: Option<RequestFilterDeviceSpec>,
    pub network_policy_device_spec: Option<NetworkPolicyDeviceSpec>,
    pub request_rate_limiting_device_spec: Option<RequestRateLimitingDeviceSpec>,
    pub wasm_device_specs: Vec<WasmDeviceSpec>,
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self {
            server_spec: ServerSpec {
                threads: Some(Located::detached(1)),
                ..Default::default()
            },
            ingress_specs: vec![],
            identity_device_spec: None,
            structured_logging_device_spec: None,
            request_filter_device_spec: None,
            network_policy_device_spec: None,
            request_rate_limiting_device_spec: None,
            wasm_device_specs: vec![],
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
            interface: Located::detached("loopback".to_string()),
            port: Located::detached(DEFAULT_LISTENER_PORT),
            tls: include_tls.then_some(Located::detached(TlsTerminationSpec::Manual {
                cert: Located::detached(PathBuf::from(CERT_SERVER_PEM)),
                key: Located::detached(PathBuf::from(CERT_SERVER_KEY)),
            })),
            ..Default::default()
        }
    }

    pub(crate) fn make_bind_with_acme() -> BindSpec {
        BindSpec {
            interface: Located::detached("loopback".to_string()),
            port: Located::detached(DEFAULT_LISTENER_PORT),
            tls: Some(Located::detached(TlsTerminationSpec::Acme {
                domains: vec![Located::detached(TEST_HOST.to_string())],
                challenge: Located::detached(AcmeChallenge::Http01.as_str().to_string()),
            })),
            ..Default::default()
        }
    }

    pub fn build(self) -> RuntimeConfig {
        match self.try_build() {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!("{e}");
                panic!("failed to load fixture config - check above for violations");
            }
        }
    }

    pub fn try_build(
        self,
    ) -> Result<RuntimeConfig, snakeway::testing_api::conf::validation::ConfigError> {
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

        if let Some(network_policy_device_spec) = self.network_policy_device_spec {
            device_specs.push(DeviceSpec::NetworkPolicy(network_policy_device_spec));
        }

        if let Some(request_rate_limiting_device_spec) = self.request_rate_limiting_device_spec {
            device_specs.push(DeviceSpec::RequestRateLimiting(
                request_rate_limiting_device_spec,
            ));
        }

        for wasm_spec in self.wasm_device_specs {
            device_specs.push(DeviceSpec::Wasm(wasm_spec));
        }

        load_config_from_specs(self.server_spec, self.ingress_specs, device_specs).map(|v| v.config)
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
            .value
            .connection_filter = Some(Located::detached(connection_filter.clone()));
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
            .value
            .connection_rate_limiting_filter = Some(Located::detached(rate_limiter.clone()));
        self
    }

    pub fn with_connection_rate_limiting_filter(
        self,
        max_connections_per_second: i64,
        window_seconds: i64,
    ) -> Self {
        let rate_limiter = ConnectionRateLimitingFilterSpec {
            max_connections_per_second: Located::detached(max_connections_per_second),
            window_seconds: Located::detached(window_seconds),
        };
        self.set_rate_limiter_on_last_bind(&rate_limiter)
    }
}
