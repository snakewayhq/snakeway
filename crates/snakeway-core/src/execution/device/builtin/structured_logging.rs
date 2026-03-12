use crate::execution::ctx::{RequestCtx, RequestId, ResponseCtx};
use crate::execution::device::core::errors::DeviceError;
use crate::execution::device::core::{Device, result::DeviceResult};
use crate::execution::enrichment::user_agent::ClientIdentity;
use crate::http_event::HttpEvent;
use anyhow::Result;
use http::HeaderMap;
use snakeway_conf::types::{
    IdentityFieldConfig, LogEventConfig, LogLevelConfig, LogPhaseConfig,
    StructuredLoggingDeviceConfig,
};
use std::collections::{BTreeMap, HashSet};
use tracing::{debug, error, info, trace, warn};

// ----------------------------------------------------------------------------
// Emit macro ...to DRY-out logging calls.
// ----------------------------------------------------------------------------

macro_rules! emit {
    ($level:expr, $($fields:tt)*) => {
        match $level {
            LogLevelConfig::Trace => trace!($($fields)*),
            LogLevelConfig::Debug => debug!($($fields)*),
            LogLevelConfig::Info  => info!($($fields)*),
            LogLevelConfig::Warn  => warn!($($fields)*),
            LogLevelConfig::Error => error!($($fields)*),
        }
    };
}

// ----------------------------------------------------------------------------
// Device implementation
// ----------------------------------------------------------------------------

pub(crate) struct StructuredLoggingDevice {
    level: LogLevelConfig,

    include_headers: bool,
    allowed_headers: HashSet<String>,
    redact_headers: HashSet<String>,

    include_identity: bool,
    identity_fields: Vec<IdentityFieldConfig>,

    events: Option<Vec<LogEventConfig>>,
    phases: Option<Vec<LogPhaseConfig>>,
}

impl StructuredLoggingDevice {
    pub(crate) fn from_config(cfg: StructuredLoggingDeviceConfig) -> Result<Self> {
        Ok(Self {
            level: cfg.level,

            include_headers: cfg.include_headers,
            allowed_headers: cfg
                .allowed_headers
                .into_iter()
                .map(|h| h.to_lowercase())
                .collect(),
            redact_headers: cfg
                .redacted_headers
                .into_iter()
                .map(|h| h.to_lowercase())
                .collect(),

            include_identity: cfg.include_identity,
            identity_fields: cfg.identity_fields,

            events: cfg.events,
            phases: cfg.phases,
        })
    }

    // ------------------------------------------------------------------------
    // Gating helpers
    // ------------------------------------------------------------------------

    fn event_enabled(&self, event: LogEventConfig) -> bool {
        self.events.as_ref().is_none_or(|e| e.contains(&event))
    }

    fn phase_enabled(&self, phase: LogPhaseConfig) -> bool {
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

    fn build_headers(&self, headers: &HeaderMap) -> BTreeMap<String, String> {
        let mut out = BTreeMap::new();

        for (name, value) in headers.iter() {
            let name_lc = name.as_str().to_lowercase();

            // Allowlist check (if configured)
            if !self.allowed_headers.is_empty() && !self.allowed_headers.contains(&name_lc) {
                continue;
            }

            let val = if self.redact_headers.contains(&name_lc) {
                "<redacted>".to_string()
            } else {
                value
                    .to_str()
                    .map(str::to_string)
                    .unwrap_or("<binary>".into())
            };

            out.insert(name_lc, val);
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

        let geo = identity.geo.as_ref();
        let ua = identity.ua.as_ref();

        let mut out: BTreeMap<String, _> = BTreeMap::new();

        for field in &self.identity_fields {
            match field {
                IdentityFieldConfig::ClientIp => {
                    out.insert("client_ip".into(), identity.ip.to_string());
                }

                IdentityFieldConfig::ProxyChain => {
                    if !identity.proxy_chain.is_empty() {
                        let chain: Vec<String> = identity
                            .proxy_chain
                            .iter()
                            .map(|ip| ip.to_string())
                            .collect();
                        out.insert("proxy_chain".into(), chain.join(","));
                    }
                }

                IdentityFieldConfig::Forwarded => {
                    out.insert("is_forwarded".into(), identity.is_forwarded.to_string());
                }

                IdentityFieldConfig::Trusted => {
                    out.insert("is_trusted".into(), identity.is_trusted.to_string());
                }

                IdentityFieldConfig::Country => {
                    if let Some(cc) = geo.and_then(|g| g.country_code.as_ref()) {
                        out.insert("country".into(), cc.clone());
                    }
                }
                IdentityFieldConfig::Region => {
                    if let Some(r) = geo.and_then(|g| g.region.as_ref()) {
                        out.insert("region".into(), r.clone());
                    }
                }
                IdentityFieldConfig::Asn => {
                    if let Some(asn) = geo.and_then(|g| g.asn) {
                        out.insert("asn".into(), asn.to_string());
                    }
                }
                IdentityFieldConfig::Aso => {
                    if let Some(aso) = geo.and_then(|g| g.aso.as_ref()) {
                        out.insert("aso".into(), aso.to_string());
                    }
                }
                IdentityFieldConfig::ConnectionType => {
                    if let Some(connection_type) = geo.and_then(|g| g.connection_type.as_ref()) {
                        out.insert("connection_type".into(), connection_type.to_string());
                    }
                }
                IdentityFieldConfig::Device => {
                    if let Some(ua) = ua {
                        out.insert("device".into(), ua.device_type.as_str().to_string());
                    }
                }
                IdentityFieldConfig::Bot => {
                    if let Some(ua) = ua {
                        out.insert("bot".into(), ua.is_bot.to_string());
                    }
                }
            }
        }

        serde_json::to_string(&out).ok()
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

// ----------------------------------------------------------------------------
// Device trait
// ----------------------------------------------------------------------------
impl Device for StructuredLoggingDevice {
    fn name(&self) -> &str {
        "Structured Logging"
    }

    fn on_request(&self, ctx: &mut RequestCtx) -> DeviceResult {
        if self.phase_enabled(LogPhaseConfig::Request)
            && self.event_enabled(LogEventConfig::Request)
        {
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
        if self.phase_enabled(LogPhaseConfig::Request)
            && self.event_enabled(LogEventConfig::BeforeProxy)
        {
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
        if self.phase_enabled(LogPhaseConfig::Response)
            && self.event_enabled(LogEventConfig::AfterProxy)
        {
            self.emit_http_response(ctx, HttpEvent::AfterProxy);
        }
        DeviceResult::Continue
    }

    fn on_response(&self, ctx: &mut ResponseCtx) -> DeviceResult {
        if self.phase_enabled(LogPhaseConfig::Response)
            && self.event_enabled(LogEventConfig::Response)
        {
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
