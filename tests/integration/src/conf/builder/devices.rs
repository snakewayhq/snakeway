use crate::conf::ConfigBuilder;
use snakeway_core::testing_api::conf::types::{
    ForwardingSpec, IdentityDeviceSpec, IdentityFieldSpec, LogEventSpec, LogLevelSpec,
    LogPhaseSpec, NetworkPolicyDeviceSpec, OnInvalidForwardedSpec, RequestFilterDeviceSpec,
    RequestRateLimitingDeviceSpec, StructuredLoggingDeviceSpec,
};

use std::path::PathBuf;

/// Identity Device
impl ConfigBuilder {
    pub fn with_identity_device(mut self, spec: IdentityDeviceSpec) -> Self {
        self.identity_device_spec = Some(spec);
        self
    }

    pub fn make_identity_device() -> IdentityDeviceSpec {
        IdentityDeviceSpec {
            enable: true,
            trusted_proxies: vec![],
            enable_geoip: true,
            geoip_city_db: Some(PathBuf::from(
                "fixtures/geoip/dbip-country-lite-2025-12.mmdb",
            )),
            enable_user_agent: true,
            max_x_forwarded_for_length: 1024,
            max_user_agent_length: 2048,
            ..Default::default()
        }
    }
}

/// Structured Logging Device
impl ConfigBuilder {
    pub fn with_structured_logging(mut self, spec: StructuredLoggingDeviceSpec) -> Self {
        self.structured_logging_device_spec = Some(spec);
        self
    }

    pub fn make_structured_logging_device() -> StructuredLoggingDeviceSpec {
        StructuredLoggingDeviceSpec {
            enable: true,
            level: LogLevelSpec::Info,
            include_headers: true,
            allowed_headers: vec![
                "user-agent".to_string(),
                "host".to_string(),
                "x-forwarded-for".to_string(),
                "x-real-ip".to_string(),
            ],
            redacted_headers: vec!["authentication".to_string(), "cookie".to_string()],
            include_identity: true,
            identity_fields: vec![
                IdentityFieldSpec::Asn,
                IdentityFieldSpec::Aso,
                IdentityFieldSpec::Bot,
                IdentityFieldSpec::Country,
                IdentityFieldSpec::Region,
                IdentityFieldSpec::Device,
                IdentityFieldSpec::ConnectionType,
            ],
            events: Some(vec![
                LogEventSpec::Request,
                LogEventSpec::BeforeProxy,
                LogEventSpec::AfterProxy,
                LogEventSpec::Response,
            ]),
            phases: Some(vec![LogPhaseSpec::Request, LogPhaseSpec::Response]),
            ..Default::default()
        }
    }
}

/// Request Filter Device
impl ConfigBuilder {
    pub fn with_request_filter(mut self, spec: RequestFilterDeviceSpec) -> Self {
        self.request_filter_device_spec = Some(spec);
        self
    }

    pub fn make_request_filter_device_spec() -> RequestFilterDeviceSpec {
        RequestFilterDeviceSpec {
            enable: true,
            allow_methods: vec!["GET".to_string(), "POST".to_string(), "DELETE".to_string()],
            deny_methods: vec![],
            deny_headers: vec!["x-forwarded-host".to_string(), "x-original-url".to_string()],
            allow_headers: vec![],
            required_headers: vec!["host".to_string()],
            max_header_bytes: 1024,          // 1 KB
            max_body_bytes: 16384,           // 16 KB
            max_suspicious_body_bytes: 1024, // 1 KB
            deny_status: None,
            ..Default::default()
        }
    }
}

/// Network Policy Device
impl ConfigBuilder {
    pub fn with_network_policy(mut self, spec: NetworkPolicyDeviceSpec) -> Self {
        self.network_policy_device_spec = Some(spec);
        self
    }

    pub fn make_network_policy_device_spec(cidrs: Vec<&str>) -> NetworkPolicyDeviceSpec {
        NetworkPolicyDeviceSpec {
            enable: true,
            cidr_allow: cidrs.into_iter().map(|c| c.to_string()).collect(),
            forwarding: ForwardingSpec {
                allow: true,
                on_invalid: OnInvalidForwardedSpec::Deny,
            },
            ..Default::default()
        }
    }
}

/// Request Rate Limiting Device
impl ConfigBuilder {
    pub fn with_request_rate_limiting(
        mut self,
        max_requests_per_second: u16,
        window_seconds: u16,
    ) -> Self {
        self.request_rate_limiting_device_spec = Some(RequestRateLimitingDeviceSpec {
            enable: true,

            max_requests_per_second,
            window_seconds,

            ..Default::default()
        });
        self
    }
}
