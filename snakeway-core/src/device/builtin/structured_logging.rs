use crate::ctx::{RequestCtx, ResponseCtx};
use crate::device::core::errors::DeviceError;
use crate::device::core::{Device, result::DeviceResult};
use crate::http_event::HttpEvent;
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

    #[serde(default)]
    include_headers: bool,

    #[serde(default)]
    redact_headers: Vec<String>,

    events: Option<Vec<LogEvent>>,
    phases: Option<Vec<LogPhase>>,
}

fn default_level() -> LogLevel {
    LogLevel::Info
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

    fn emit_http_event(
        &self,
        event: HttpEvent,
        method: Option<&str>,
        uri: Option<&str>,
        status: Option<&str>,
        headers: &HeaderMap,
    ) {
        match self.headers_json(headers) {
            Some(headers) => {
                emit!(
                    self.level,
                    event = %event.as_str(),
                    method = method,
                    uri = uri,
                    status = status,
                    headers = %headers,
                );
            }
            None => {
                emit!(
                    self.level,
                    event = %event.as_str(),
                    method = method,
                    uri = uri,
                    status = status,
                );
            }
        }
    }
}

impl Device for StructuredLoggingDevice {
    fn on_request(&self, ctx: &mut RequestCtx) -> DeviceResult {
        if !self.phase_enabled(LogPhase::Request) || !self.event_enabled(LogEvent::Request) {
            return DeviceResult::Continue;
        }

        self.emit_http_event(
            HttpEvent::Request,
            Some(ctx.method.as_str()),
            Some(&ctx.original_uri.to_string()),
            None,
            &ctx.headers,
        );

        DeviceResult::Continue
    }

    fn before_proxy(&self, ctx: &mut RequestCtx) -> DeviceResult {
        if !self.phase_enabled(LogPhase::Request) || !self.event_enabled(LogEvent::BeforeProxy) {
            return DeviceResult::Continue;
        }

        self.emit_http_event(
            HttpEvent::BeforeProxy,
            Some(ctx.method.as_str()),
            Some(&ctx.original_uri.to_string()),
            None,
            &ctx.headers,
        );

        DeviceResult::Continue
    }

    fn after_proxy(&self, ctx: &mut ResponseCtx) -> DeviceResult {
        if !self.phase_enabled(LogPhase::Response) || !self.event_enabled(LogEvent::AfterProxy) {
            return DeviceResult::Continue;
        }

        self.emit_http_event(
            HttpEvent::AfterProxy,
            None,
            None,
            Some(ctx.status.as_str()),
            &ctx.headers,
        );

        DeviceResult::Continue
    }

    fn on_response(&self, ctx: &mut ResponseCtx) -> DeviceResult {
        if !self.phase_enabled(LogPhase::Response) || !self.event_enabled(LogEvent::Response) {
            return DeviceResult::Continue;
        }

        self.emit_http_event(
            HttpEvent::Response,
            None,
            None,
            Some(ctx.status.as_str()),
            &ctx.headers,
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
