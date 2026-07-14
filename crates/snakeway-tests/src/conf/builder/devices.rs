use crate::conf::ConfigBuilder;
use confval::source::Located;
use snakeway::testing_api::conf::types::{
    ForwardingSpec, IdentityDeviceSpec, NetworkPolicyDeviceSpec, RequestFilterDeviceSpec,
    RequestRateLimitingDeviceSpec, StructuredLoggingDeviceSpec, WasmDeviceSpec,
};

use std::path::PathBuf;

fn located_list(values: &[&str]) -> Vec<Located<String>> {
    values
        .iter()
        .map(|v| Located::detached(v.to_string()))
        .collect()
}

impl ConfigBuilder {
    pub fn with_identity_device(mut self, spec: IdentityDeviceSpec) -> Self {
        self.identity_device_spec = Some(spec);
        self
    }

    pub fn make_identity_device() -> IdentityDeviceSpec {
        IdentityDeviceSpec {
            enable: Located::detached(true),
            trusted_proxies: vec![],
            enable_geoip: Located::detached(true),
            geoip_city_db: Some(Located::detached(PathBuf::from(
                "fixtures/geoip/dbip-country-lite-2025-12.mmdb",
            ))),
            enable_user_agent: Located::detached(true),
            max_x_forwarded_for_length: Located::detached(1024),
            max_user_agent_length: Located::detached(2048),
            ..Default::default()
        }
    }

    pub fn make_identity_device_with_trusted_proxy() -> IdentityDeviceSpec {
        let mut spec = Self::make_identity_device();
        spec.trusted_proxies = located_list(&["127.0.0.1/32"]);
        spec
    }
}

impl ConfigBuilder {
    pub fn with_structured_logging(mut self, spec: StructuredLoggingDeviceSpec) -> Self {
        self.structured_logging_device_spec = Some(spec);
        self
    }

    pub fn make_structured_logging_device() -> StructuredLoggingDeviceSpec {
        StructuredLoggingDeviceSpec {
            enable: Located::detached(true),
            level: Located::detached("info".to_string()),
            include_headers: Located::detached(true),
            allowed_headers: located_list(&["user-agent", "host", "x-forwarded-for", "x-real-ip"]),
            redacted_headers: located_list(&["authentication", "cookie"]),
            include_identity: Located::detached(true),
            identity_fields: located_list(&[
                "asn",
                "aso",
                "bot",
                "country",
                "region",
                "device",
                "connection_type",
            ]),
            events: Some(Located::detached(located_list(&[
                "request",
                "before_proxy",
                "after_proxy",
                "response",
            ]))),
            phases: Some(Located::detached(located_list(&["request", "response"]))),
        }
    }
}

impl ConfigBuilder {
    pub fn with_request_filter(mut self, spec: RequestFilterDeviceSpec) -> Self {
        self.request_filter_device_spec = Some(spec);
        self
    }

    pub fn make_request_filter_device_spec() -> RequestFilterDeviceSpec {
        RequestFilterDeviceSpec {
            enable: Located::detached(true),
            allow_methods: located_list(&["GET", "POST", "DELETE"]),
            deny_methods: vec![],
            deny_headers: located_list(&["x-forwarded-host", "x-original-url"]),
            allow_headers: vec![],
            required_headers: located_list(&["host"]),
            max_header_bytes: Located::detached(1024), // 1 KB
            max_body_bytes: Located::detached(16384),  // 16 KB
            max_suspicious_body_bytes: Located::detached(1024), // 1 KB
            deny_status: None,
            ..Default::default()
        }
    }
}

impl ConfigBuilder {
    pub fn with_network_policy(mut self, spec: NetworkPolicyDeviceSpec) -> Self {
        self.network_policy_device_spec = Some(spec);
        self
    }

    pub fn make_network_policy_device_spec(cidrs: Vec<&str>) -> NetworkPolicyDeviceSpec {
        NetworkPolicyDeviceSpec {
            enable: Located::detached(true),
            cidr_allow: cidrs
                .into_iter()
                .map(|c| Located::detached(c.to_string()))
                .collect(),
            forwarding: Located::detached(ForwardingSpec {
                allow: Located::detached(true),
                on_invalid: Located::detached("deny".to_string()),
            }),
            ..Default::default()
        }
    }
}

/// Request Rate Limiting Device
impl ConfigBuilder {
    pub fn with_request_rate_limiting(
        mut self,
        max_requests_per_second: i64,
        window_seconds: i64,
    ) -> Self {
        self.request_rate_limiting_device_spec = Some(RequestRateLimitingDeviceSpec {
            enable: Located::detached(true),

            max_requests_per_second: Located::detached(max_requests_per_second),
            window_seconds: Located::detached(window_seconds),

            ..Default::default()
        });
        self
    }
}

impl ConfigBuilder {
    pub fn with_wasm_device(mut self, spec: WasmDeviceSpec) -> Self {
        self.wasm_device_specs.push(spec);
        self
    }
}
