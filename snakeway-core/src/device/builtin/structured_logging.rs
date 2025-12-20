use crate::ctx::{RequestCtx, ResponseCtx};
use crate::device::core::errors::DeviceError;
use crate::device::core::{Device, result::DeviceResult};
use crate::http_event::HttpEvent;
use crate::user_agent::ClientIdentity;
use anyhow::{Context, Result};
use http::HeaderMap;
use serde::Deserialize;
use std::collections::BTreeMap;
use tracing::{debug, error, info, trace, warn};
// ----------------------------------------------------------------------------
// Logging level & config enums
// ----------------------------------------------------------------------------

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum LogEvent {
    Request,
    BeforeProxy,
    AfterProxy,
    Response,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum LogPhase {
    Request,
    Response,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LoggingConfig {
    #[serde(default = "default_level")]
    level: LogLevel,

    // Headers are excluded by default for EU compliance reasons.
    #[serde(default)]
    include_headers: bool,

    #[serde(default)]
    redact_headers: Vec<String>,

    // Identity logging (EU-safe)
    #[serde(default)]
    include_identity: bool,

    #[serde(default)]
    identity_fields: Vec<IdentityField>,

    events: Option<Vec<LogEvent>>,
    phases: Option<Vec<LogPhase>>,
}

fn default_level() -> LogLevel {
    LogLevel::Info
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum IdentityField {
    Country,
    Region,
    Device,
    Bot,
    Asn,
}

fn default_identity_fields() -> Vec<IdentityField> {
    vec![IdentityField::Country, IdentityField::Device]
}

// ----------------------------------------------------------------------------
// Emit macro (DRY-out logging calls)
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
    redact_headers: Vec<String>,
    include_identity: bool,
    identity_fields: Vec<IdentityField>,
    events: Option<Vec<LogEvent>>,
    phases: Option<Vec<LogPhase>>,
}

impl StructuredLoggingDevice {
    pub fn from_config(raw: &toml::Value) -> Result<Self> {
        let cfg: LoggingConfig = raw
            .clone()
            .try_into()
            .context("invalid structured_logging config")?;

        Ok(Self {
            level: cfg.level,
            include_headers: cfg.include_headers,
            redact_headers: cfg
                .redact_headers
                .into_iter()
                .map(|h| h.to_lowercase())
                .collect(),

            include_identity: cfg.include_identity,
            identity_fields: if cfg.identity_fields.is_empty() {
                default_identity_fields()
            } else {
                cfg.identity_fields
            },

            events: cfg.events,
            phases: cfg.phases,
        })
    }

    // Gating helpers
    fn event_enabled(&self, event: LogEvent) -> bool {
        match &self.events {
            Some(events) => events.contains(&event),
            None => true,
        }
    }

    fn phase_enabled(&self, phase: LogPhase) -> bool {
        match &self.phases {
            Some(phases) => phases.contains(&phase),
            None => true,
        }
    }

    fn headers_json(&self, headers: &HeaderMap) -> Option<String> {
        if !self.include_headers {
            return None;
        }

        let headers = self.build_redacted_headers(headers);

        serde_json::to_string(&headers).ok()
    }

    fn build_redacted_headers(&self, headers: &HeaderMap) -> BTreeMap<String, String> {
        let mut out = BTreeMap::new();

        for (name, value) in headers.iter() {
            let name_str = name.as_str().to_lowercase();

            let redacted = self.redact_headers.contains(&name_str);

            let val = if redacted {
                "<redacted>".to_string()
            } else {
                match value.to_str() {
                    Ok(v) => v.to_string(),
                    Err(_) => "<binary>".to_string(),
                }
            };

            out.insert(name_str, val);
        }

        out
    }

    fn identity_json(&self, client_identity: &ClientIdentity) -> Option<String> {
        if !self.include_identity {
            return None;
        }

        let mut out: BTreeMap<String, String> = BTreeMap::new();

        for field in &self.identity_fields {
            match field {
                IdentityField::Country => {
                    if let Some(geo) = &client_identity.geo {
                        if let Some(cc) = &geo.country_code {
                            out.insert("country".into(), cc.clone());
                        }
                    }
                }
                IdentityField::Region => {
                    if let Some(geo) = &client_identity.geo {
                        if let Some(r) = &geo.region {
                            out.insert("region".into(), r.clone());
                        }
                    }
                }
                IdentityField::Device => {
                    if let Some(ua) = &client_identity.ua {
                        out.insert("device".into(), ua.device_type.as_str().to_string());
                    }
                }
                IdentityField::Bot => {
                    if let Some(ua) = &client_identity.ua {
                        out.insert("bot".into(), ua.is_bot.to_string());
                    }
                }
                IdentityField::Asn => {
                    if let Some(geo) = &client_identity.geo {
                        if let Some(asn) = geo.asn {
                            out.insert("asn".into(), asn.to_string());
                        }
                    }
                }
            }
        }

        serde_json::to_string(&out).ok()
    }

    fn emit_http_request_log_event(
        &self,
        ctx: &RequestCtx,
        event: HttpEvent,
        method: Option<&str>,
        uri: Option<&str>,
        status: Option<&str>,
    ) {
        let identity = match ctx.extensions.get::<ClientIdentity>() {
            None => None,
            Some(client_identity) => self.identity_json(client_identity),
        };

        emit!(
            self.level,
            event = %event.as_str(),
            method = method,
            uri = uri,
            status = status,
            identity = identity,
        );
    }

    fn emit_http_response_log_event(
        &self,
        event: HttpEvent,
        method: Option<&str>,
        uri: Option<&str>,
        status: Option<&str>,
    ) {
        emit!(
            self.level,
            event = %event.as_str(),
            method = method,
            uri = uri,
            status = status,
        );
    }
}

impl Device for StructuredLoggingDevice {
    fn on_request(&self, ctx: &mut RequestCtx) -> DeviceResult {
        if !self.phase_enabled(LogPhase::Request) || !self.event_enabled(LogEvent::Request) {
            return DeviceResult::Continue;
        }

        self.emit_http_request_log_event(
            ctx,
            HttpEvent::Request,
            Some(ctx.method.as_str()),
            Some(&ctx.original_uri.to_string()),
            None,
        );

        DeviceResult::Continue
    }

    fn before_proxy(&self, ctx: &mut RequestCtx) -> DeviceResult {
        if !self.phase_enabled(LogPhase::Request) || !self.event_enabled(LogEvent::BeforeProxy) {
            return DeviceResult::Continue;
        }

        self.emit_http_request_log_event(
            ctx,
            HttpEvent::BeforeProxy,
            Some(ctx.method.as_str()),
            Some(&ctx.original_uri.to_string()),
            None,
        );

        DeviceResult::Continue
    }

    fn after_proxy(&self, ctx: &mut ResponseCtx) -> DeviceResult {
        if !self.phase_enabled(LogPhase::Response) || !self.event_enabled(LogEvent::AfterProxy) {
            return DeviceResult::Continue;
        }

        self.emit_http_response_log_event(
            HttpEvent::AfterProxy,
            None,
            None,
            Some(ctx.status.as_str()),
        );

        DeviceResult::Continue
    }

    fn on_response(&self, ctx: &mut ResponseCtx) -> DeviceResult {
        if !self.phase_enabled(LogPhase::Response) || !self.event_enabled(LogEvent::Response) {
            return DeviceResult::Continue;
        }

        self.emit_http_response_log_event(
            HttpEvent::Response,
            None,
            None,
            Some(ctx.status.as_str()),
        );

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
