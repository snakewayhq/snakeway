use crate::conf::types::StructuredLoggingDeviceConfig;
use crate::ctx::{RequestCtx, ResponseCtx};
use crate::device::core::errors::DeviceError;
use crate::device::core::{Device, result::DeviceResult};
use crate::enrichment::user_agent::ClientIdentity;
use crate::http_event::HttpEvent;
use anyhow::Result;
use http::HeaderMap;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use tracing::{debug, error, info, trace, warn};

// ----------------------------------------------------------------------------
// Logging level & config enums
// ----------------------------------------------------------------------------

#[derive(Default, Debug, Deserialize, Serialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    #[default]
    Error,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LogEvent {
    Request,
    BeforeProxy,
    AfterProxy,
    Response,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogPhase {
    Request,
    Response,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IdentityField {
    Country,
    Region,
    Device,
    Bot,
    Asn,
}

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

pub struct StructuredLoggingDevice {
    level: LogLevel,

    include_headers: bool,
    allowed_headers: HashSet<String>,
    redact_headers: HashSet<String>,

    include_identity: bool,
    identity_fields: Vec<IdentityField>,

    events: Option<Vec<LogEvent>>,
    phases: Option<Vec<LogPhase>>,
}

impl StructuredLoggingDevice {
    pub fn from_config(cfg: StructuredLoggingDeviceConfig) -> Result<Self> {
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
                IdentityField::Country => {
                    if let Some(cc) = geo.and_then(|g| g.country_code.as_ref()) {
                        out.insert("country".into(), cc.clone());
                    }
                }
                IdentityField::Region => {
                    if let Some(r) = geo.and_then(|g| g.region.as_ref()) {
                        out.insert("region".into(), r.clone());
                    }
                }
                IdentityField::Asn => {
                    if let Some(asn) = geo.and_then(|g| g.asn) {
                        out.insert("asn".into(), asn.to_string());
                    }
                }
                IdentityField::Device => {
                    if let Some(ua) = ua {
                        out.insert("device".into(), ua.device_type.as_str().to_string());
                    }
                }
                IdentityField::Bot => {
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
        method: Option<&str>,
        uri: Option<&str>,
        status: Option<&str>,
    ) {
        let headers = self.headers_json(&ctx.headers);
        let identity = ctx
            .extensions
            .get::<ClientIdentity>()
            .and_then(|i| self.identity_json(i));

        emit!(
            self.level,
            event = %event.as_str(),
            method = method,
            uri = uri,
            status = status,
            headers = headers,
            identity = identity,
        );
    }

    fn emit_http_response(&self, event: HttpEvent, status: Option<&str>) {
        emit!(
            self.level,
            event = %event.as_str(),
            status = status,
        );
    }
}

// ----------------------------------------------------------------------------
// Device trait
// ----------------------------------------------------------------------------
impl Device for StructuredLoggingDevice {
    fn on_request(&self, ctx: &mut RequestCtx) -> DeviceResult {
        if self.phase_enabled(LogPhase::Request) && self.event_enabled(LogEvent::Request) {
            self.emit_http_request(
                ctx,
                HttpEvent::Request,
                ctx.method_str(),
                ctx.original_uri_str().as_deref(),
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
                ctx.original_uri_str().as_deref(),
                None,
            );
        }
        DeviceResult::Continue
    }

    fn after_proxy(&self, ctx: &mut ResponseCtx) -> DeviceResult {
        if self.phase_enabled(LogPhase::Response) && self.event_enabled(LogEvent::AfterProxy) {
            self.emit_http_response(HttpEvent::AfterProxy, Some(ctx.status.as_str()));
        }
        DeviceResult::Continue
    }

    fn on_response(&self, ctx: &mut ResponseCtx) -> DeviceResult {
        if self.phase_enabled(LogPhase::Response) && self.event_enabled(LogEvent::Response) {
            self.emit_http_response(HttpEvent::Response, Some(ctx.status.as_str()));
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
