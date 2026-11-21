use crate::execution::ctx::{RequestCtx, RequestId, ResponseCtx};
use crate::execution::device::core::{Device, DeviceError, DeviceResult};
use crate::execution::enrichment::user_agent::ClientIdentity;

#[derive(Debug, Clone, Copy)]
enum HttpEvent {
    Request,
    BeforeProxy,
    AfterProxy,
    Response,
}

impl HttpEvent {
    fn as_str(self) -> &'static str {
        match self {
            HttpEvent::Request => "request",
            HttpEvent::BeforeProxy => "before_proxy",
            HttpEvent::AfterProxy => "after_proxy",
            HttpEvent::Response => "response",
        }
    }
}
use http::HeaderMap;
use snakeway_conf::types::{
    IdentityField, LogEvent, LogLevel, LogPhase, StructuredLoggingDeviceConfig,
};
use std::borrow::Cow;
use std::collections::{BTreeMap, HashSet};
use std::fmt::Write as _;
use tracing::{debug, error, info, trace, warn};

// ----------------------------------------------------------------------------
// Emit macro ...to DRY-out logging calls.
// ----------------------------------------------------------------------------

macro_rules! emit {
    ($level:expr, $($fields:tt)*) => {
        match $level {
            LogLevel::Trace => trace!($($fields)*),
            LogLevel::Debug => debug!($($fields)*),
            LogLevel::Info  => info!($($fields)*),
            LogLevel::Warn  => warn!($($fields)*),
            LogLevel::Error => error!($($fields)*),
        }
    };
}

// ----------------------------------------------------------------------------
// Device implementation
// ----------------------------------------------------------------------------

pub(crate) struct StructuredLoggingDevice {
    level: LogLevel,

    include_headers: bool,
    allowed_headers: HashSet<String>,
    redacted_headers: HashSet<String>,

    include_identity: bool,
    identity_fields: Vec<IdentityField>,

    events: Option<Vec<LogEvent>>,
    phases: Option<Vec<LogPhase>>,
}

impl StructuredLoggingDevice {
    // ------------------------------------------------------------------------
    // Gating helpers
    // ------------------------------------------------------------------------

    fn event_enabled(&self, event: LogEvent) -> bool {
        self.events.as_ref().is_none_or(|e| e.contains(&event))
    }

    fn phase_enabled(&self, phase: LogPhase) -> bool {
        self.phases.as_ref().is_none_or(|p| p.contains(&phase))
    }

    // ------------------------------------------------------------------------
    // Header handling
    // ------------------------------------------------------------------------

    fn headers_json(&self, headers: &HeaderMap) -> Option<String> {
        if !self.include_headers {
            return None;
        }

        let map = self.build_headers(headers);
        serde_json::to_string(&map).ok()
    }

    fn build_headers<'a>(&self, headers: &'a HeaderMap) -> BTreeMap<&'a str, &'a str> {
        let mut out = BTreeMap::new();

        for (name, value) in headers.iter() {
            // A header name is always lowercase, so it compares directly against the sets.
            let name = name.as_str();

            // Allowlist check (if configured)
            if !self.allowed_headers.is_empty() && !self.allowed_headers.contains(name) {
                continue;
            }

            let val = if self.redacted_headers.contains(name) {
                "<redacted>"
            } else {
                value.to_str().unwrap_or("<binary>")
            };

            out.insert(name, val);
        }

        out
    }

    // ------------------------------------------------------------------------
    // Identity handling
    // ------------------------------------------------------------------------

    fn identity_json(&self, identity: &ClientIdentity) -> Option<String> {
        if !self.include_identity {
            return None;
        }

        serde_json::to_string(&self.identity_map(identity)).ok()
    }

    fn identity_map<'a>(
        &self,
        identity: &'a ClientIdentity,
    ) -> BTreeMap<&'static str, Cow<'a, str>> {
        let geo = identity.geo.as_ref();
        let ua = identity.ua.as_ref();

        let mut out = BTreeMap::new();

        for field in &self.identity_fields {
            match field {
                IdentityField::ClientIp => {
                    out.insert("client_ip", Cow::Owned(identity.ip.to_string()));
                }

                IdentityField::ProxyChain => {
                    if !identity.proxy_chain.is_empty() {
                        let mut chain = String::with_capacity(identity.proxy_chain.len() * 16);
                        for (index, ip) in identity.proxy_chain.iter().enumerate() {
                            if index > 0 {
                                chain.push(',');
                            }
                            let _ = write!(chain, "{ip}");
                        }
                        out.insert("proxy_chain", Cow::Owned(chain));
                    }
                }

                IdentityField::Forwarded => {
                    out.insert(
                        "is_forwarded",
                        Cow::Borrowed(bool_str(identity.is_forwarded)),
                    );
                }

                IdentityField::Trusted => {
                    out.insert("is_trusted", Cow::Borrowed(bool_str(identity.is_trusted)));
                }

                IdentityField::Country => {
                    if let Some(cc) = geo.and_then(|g| g.country_code.as_deref()) {
                        out.insert("country", Cow::Borrowed(cc));
                    }
                }
                IdentityField::Region => {
                    if let Some(r) = geo.and_then(|g| g.region.as_deref()) {
                        out.insert("region", Cow::Borrowed(r));
                    }
                }
                IdentityField::Asn => {
                    if let Some(asn) = geo.and_then(|g| g.asn) {
                        out.insert("asn", Cow::Owned(asn.to_string()));
                    }
                }
                IdentityField::Aso => {
                    if let Some(aso) = geo.and_then(|g| g.aso.as_deref()) {
                        out.insert("aso", Cow::Borrowed(aso));
                    }
                }
                IdentityField::ConnectionType => {
                    if let Some(connection_type) = geo.and_then(|g| g.connection_type.as_deref()) {
                        out.insert("connection_type", Cow::Borrowed(connection_type));
                    }
                }
                IdentityField::Device => {
                    if let Some(ua) = ua {
                        out.insert("device", Cow::Borrowed(ua.device_type.as_str()));
                    }
                }
                IdentityField::Bot => {
                    if let Some(ua) = ua {
                        out.insert("bot", Cow::Borrowed(bool_str(ua.is_bot)));
                    }
                }
            }
        }

        out
    }

    // ------------------------------------------------------------------------
    // Emit helpers
    // ------------------------------------------------------------------------

    fn emit_http_request(
        &self,
        ctx: &RequestCtx,
        event: HttpEvent,
        method: &str,
        uri: &str,
        status: Option<&str>,
    ) {
        let headers = self.headers_json(ctx.headers());
        let identity = ctx
            .extensions
            .get::<ClientIdentity>()
            .and_then(|i| self.identity_json(i));

        let request_id = self.request_id(ctx);

        emit!(
            self.level,
            event = %event.as_str(),
            request_id,
            method = method,
            uri = uri,
            status = status,
            headers = headers,
            identity = identity,
        );
    }

    fn emit_http_response(&self, ctx: &ResponseCtx, event: HttpEvent) {
        emit!(
            self.level,
            event = %event.as_str(),
            request_id = ctx.request_id.as_deref(),
            status = Some(ctx.status.as_str()),
        );
    }

    fn request_id<'a>(&self, ctx: &'a RequestCtx) -> Option<&'a str> {
        ctx.extensions
            .get::<RequestId>()
            .map(move |id| id.0.as_str())
    }
}

impl From<StructuredLoggingDeviceConfig> for StructuredLoggingDevice {
    fn from(cfg: StructuredLoggingDeviceConfig) -> Self {
        Self {
            level: cfg.level,
            include_headers: cfg.include_headers,
            allowed_headers: cfg.allowed_headers,
            redacted_headers: cfg.redacted_headers,
            include_identity: cfg.include_identity,
            identity_fields: cfg.identity_fields,
            events: cfg.events,
            phases: cfg.phases,
        }
    }
}

// ----------------------------------------------------------------------------
// Device trait
// ----------------------------------------------------------------------------
impl Device for StructuredLoggingDevice {
    fn name(&self) -> &str {
        "Structured Logging"
    }

    fn on_request(&self, ctx: &mut RequestCtx) -> DeviceResult {
        if self.phase_enabled(LogPhase::Request) && self.event_enabled(LogEvent::Request) {
            self.emit_http_request(
                ctx,
                HttpEvent::Request,
                ctx.method_str(),
                ctx.original_uri_string().as_str(),
                None,
            );
        }
        DeviceResult::Continue
    }

    fn before_proxy(&self, ctx: &mut RequestCtx) -> DeviceResult {
        if self.phase_enabled(LogPhase::Request) && self.event_enabled(LogEvent::BeforeProxy) {
            self.emit_http_request(
                ctx,
                HttpEvent::BeforeProxy,
                ctx.method_str(),
                ctx.original_uri_string().as_str(),
                None,
            );
        }
        DeviceResult::Continue
    }

    fn after_proxy(&self, ctx: &mut ResponseCtx) -> DeviceResult {
        if self.phase_enabled(LogPhase::Response) && self.event_enabled(LogEvent::AfterProxy) {
            self.emit_http_response(ctx, HttpEvent::AfterProxy);
        }
        DeviceResult::Continue
    }

    fn on_response(&self, ctx: &mut ResponseCtx) -> DeviceResult {
        if self.phase_enabled(LogPhase::Response) && self.event_enabled(LogEvent::Response) {
            self.emit_http_response(ctx, HttpEvent::Response);
        }
        DeviceResult::Continue
    }

    fn on_error(&self, err: &DeviceError) {
        emit!(
            self.level,
            event = "device_error",
            fatal = err.fatal,
            message = %err.message,
        );
    }
}

fn bool_str(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::enrichment::user_agent::{DeviceType, GeoInfo, UserAgentInfo};
    use http::{HeaderName, HeaderValue};

    fn structured_logging_device_with(
        allowed: &[&str],
        redacted: &[&str],
        fields: Vec<IdentityField>,
    ) -> StructuredLoggingDevice {
        StructuredLoggingDevice {
            level: LogLevel::Info,
            include_headers: true,
            allowed_headers: allowed.iter().map(|s| s.to_string()).collect(),
            redacted_headers: redacted.iter().map(|s| s.to_string()).collect(),
            include_identity: true,
            identity_fields: fields,
            events: None,
            phases: None,
        }
    }

    fn header_map_from_pairs(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (name, value) in pairs {
            map.append(
                HeaderName::from_bytes(name.as_bytes()).unwrap(),
                HeaderValue::from_str(value).unwrap(),
            );
        }
        map
    }

    #[test]
    fn an_allowlisted_header_matches_whatever_case_the_client_sent() {
        // Arrange
        let device = structured_logging_device_with(&["x-foo"], &[], vec![]);
        let headers = header_map_from_pairs(&[("X-Foo", "bar"), ("x-other", "baz")]);

        // Act
        let json = device.headers_json(&headers);

        // Assert
        assert_eq!(json.as_deref(), Some(r#"{"x-foo":"bar"}"#));
    }

    #[test]
    fn a_redacted_header_is_logged_as_redacted() {
        // Arrange
        let device = structured_logging_device_with(&[], &["authorization"], vec![]);
        let headers =
            header_map_from_pairs(&[("Authorization", "Bearer secret"), ("host", "a.test")]);

        // Act
        let json = device.headers_json(&headers);

        // Assert
        assert_eq!(
            json.as_deref(),
            Some(r#"{"authorization":"<redacted>","host":"a.test"}"#)
        );
    }

    #[test]
    fn a_header_value_that_is_not_text_is_logged_as_binary() {
        // Arrange
        let device = structured_logging_device_with(&[], &[], vec![]);
        let mut headers = HeaderMap::new();
        headers.insert("x-raw", HeaderValue::from_bytes(&[0x80]).unwrap());

        // Act
        let json = device.headers_json(&headers);

        // Assert
        assert_eq!(json.as_deref(), Some(r#"{"x-raw":"<binary>"}"#));
    }

    #[test]
    fn building_the_header_map_allocates_only_the_map() {
        // Arrange
        let device = structured_logging_device_with(&[], &[], vec![]);
        let headers = header_map_from_pairs(&[("x-a", "1"), ("x-b", "2"), ("x-c", "3")]);

        // Act
        let (map, allocations) =
            crate::alloc_count::count_allocations(|| device.build_headers(&headers));

        // Assert
        assert_eq!(allocations, 1);
        assert_eq!(
            serde_json::to_string(&map).unwrap(),
            r#"{"x-a":"1","x-b":"2","x-c":"3"}"#
        );
    }

    #[test]
    fn identity_json_carries_every_selected_field() {
        // Arrange
        let device = structured_logging_device_with(
            &[],
            &[],
            vec![
                IdentityField::ClientIp,
                IdentityField::ProxyChain,
                IdentityField::Forwarded,
                IdentityField::Trusted,
                IdentityField::Country,
                IdentityField::Region,
                IdentityField::Asn,
                IdentityField::Aso,
                IdentityField::ConnectionType,
                IdentityField::Device,
                IdentityField::Bot,
            ],
        );
        let identity = ClientIdentity {
            ip: "203.0.113.7".parse().unwrap(),
            proxy_chain: vec!["10.0.0.1".parse().unwrap(), "10.0.0.2".parse().unwrap()],
            is_forwarded: true,
            is_trusted: false,
            geo: Some(GeoInfo {
                country_code: Some("US".into()),
                region: Some("CA".into()),
                asn: Some(64500),
                aso: Some("Example".into()),
                connection_type: Some("Cable".into()),
            }),
            ua: Some(UserAgentInfo {
                device_type: DeviceType::Mobile,
                is_bot: false,
            }),
        };

        // Act
        let json = device.identity_json(&identity);

        // Assert
        assert_eq!(
            json.as_deref(),
            Some(concat!(
                r#"{"asn":"64500","aso":"Example","bot":"false","client_ip":"203.0.113.7","#,
                r#""connection_type":"Cable","country":"US","device":"mobile","#,
                r#""is_forwarded":"true","is_trusted":"false","#,
                r#""proxy_chain":"10.0.0.1,10.0.0.2","region":"CA"}"#
            ))
        );
    }

    #[test]
    fn identity_json_skips_fields_that_have_no_value() {
        // Arrange
        let device = structured_logging_device_with(
            &[],
            &[],
            vec![
                IdentityField::Country,
                IdentityField::Device,
                IdentityField::ClientIp,
            ],
        );
        let identity = ClientIdentity {
            ip: "203.0.113.7".parse().unwrap(),
            proxy_chain: vec![],
            is_forwarded: false,
            is_trusted: false,
            geo: None,
            ua: None,
        };

        // Act
        let json = device.identity_json(&identity);

        // Assert
        assert_eq!(json.as_deref(), Some(r#"{"client_ip":"203.0.113.7"}"#));
    }

    #[test]
    fn identity_fields_that_already_hold_text_allocate_only_the_map() {
        // Arrange
        let device = structured_logging_device_with(
            &[],
            &[],
            vec![
                IdentityField::Country,
                IdentityField::Region,
                IdentityField::Aso,
                IdentityField::ConnectionType,
                IdentityField::Device,
                IdentityField::Bot,
                IdentityField::Forwarded,
                IdentityField::Trusted,
            ],
        );
        let identity = ClientIdentity {
            ip: "203.0.113.7".parse().unwrap(),
            proxy_chain: vec![],
            is_forwarded: true,
            is_trusted: false,
            geo: Some(GeoInfo {
                country_code: Some("US".into()),
                region: Some("CA".into()),
                asn: None,
                aso: Some("Example".into()),
                connection_type: Some("Cable".into()),
            }),
            ua: Some(UserAgentInfo {
                device_type: DeviceType::Mobile,
                is_bot: false,
            }),
        };

        // Act
        let (map, allocations) =
            crate::alloc_count::count_allocations(|| device.identity_map(&identity));

        // Assert
        assert_eq!(allocations, 1);
        assert_eq!(map.len(), 8);
        assert!(map.values().all(|value| matches!(value, Cow::Borrowed(_))));
    }
}
