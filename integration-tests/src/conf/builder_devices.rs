use crate::conf::ConfigBuilder;
use snakeway_core::conf::types::{
    IdentityDeviceSpec, RequestFilterDeviceSpec, StructuredLoggingDeviceSpec,
};
use snakeway_core::device::builtin::structured_logging::{
    IdentityField, LogEvent, LogLevel, LogPhase,
};
use std::path::PathBuf;

/// Identity Device
impl ConfigBuilder {
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

    pub(crate) fn make_identity_device() -> IdentityDeviceSpec {
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
}

/// Structured Logging Device
impl ConfigBuilder {
    pub fn with_structured_logging_device(mut self) -> Self {
        self.structured_logging_device_spec = Some(Self::make_structured_logging_device());
        self
    }

    pub fn make_structured_logging_device() -> StructuredLoggingDeviceSpec {
        StructuredLoggingDeviceSpec {
            enable: true,
            level: LogLevel::Info,
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
                IdentityField::Asn,
                IdentityField::Aso,
                IdentityField::Bot,
                IdentityField::Country,
                IdentityField::Region,
                IdentityField::Device,
                IdentityField::ConnectionType,
            ],
            events: Some(vec![
                LogEvent::Request,
                LogEvent::BeforeProxy,
                LogEvent::AfterProxy,
                LogEvent::Response,
            ]),
            phases: Some(vec![LogPhase::Request, LogPhase::Response]),
            ..Default::default()
        }
    }
}

/// Request Filter Device
impl ConfigBuilder {
    pub fn with_request_filter_device(mut self) -> Self {
        self.request_filter_device_spec = Some(Self::make_request_filter_device_spec());
        self
    }

    pub fn with_request_filter_device_that_denies_get_method(mut self) -> Self {
        let mut device_spec = Self::make_request_filter_device_spec();
        device_spec.deny_methods = vec!["GET".to_string()];
        self.request_filter_device_spec = Some(device_spec);
        self
    }

    pub fn with_request_filter_device_that_requires_header(mut self) -> Self {
        let mut device_spec = Self::make_request_filter_device_spec();
        device_spec.required_headers = vec!["x-required".to_string()];
        self.request_filter_device_spec = Some(device_spec);
        self
    }

    pub fn with_request_filter_device_that_allows_specific_headers(mut self) -> Self {
        let mut device_spec = Self::make_request_filter_device_spec();
        device_spec.allow_headers = vec![
            "Host".to_string(),
            "X-Custom-Allowed".to_string(),
            "Accept".to_string(),
            "Accept-Encoding".to_string(),
            "User-Agent".to_string(),
            "Content-Length".to_string(),
        ];
        self.request_filter_device_spec = Some(device_spec);
        self
    }

    pub fn with_request_filter_device_that_overrides_deny_status(mut self) -> Self {
        let mut device_spec = Self::make_request_filter_device_spec();
        device_spec.deny_methods = vec!["DELETE".to_string()];
        device_spec.deny_status = Some(406);
        self.request_filter_device_spec = Some(device_spec);
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
