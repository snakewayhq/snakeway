use std::collections::BTreeMap;
use crate::ctx::{RequestCtx, ResponseCtx};
use crate::device::core::{result::DeviceResult, Device};
use anyhow::{Context, Result};
use http::HeaderMap;
use serde::Deserialize;
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

    fn maybe_headers(&self, headers: &HeaderMap) -> Option<BTreeMap<String, String>> {
        if self.include_headers {
            Some(self.redact_headers(headers))
        } else {
            None
        }
    }

    fn redact_headers(&self, headers: &HeaderMap) -> BTreeMap<String, String> {
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
}

impl Device for StructuredLoggingDevice {
    fn on_request(&self, ctx: &mut RequestCtx) -> DeviceResult {
        if !self.phase_enabled(LogPhase::Request) || !self.event_enabled(LogEvent::Request) {
            return DeviceResult::Continue;
        }

        let headers = self.maybe_headers(&ctx.headers);

        emit!(
            self.level,
            event = "request",
            method = %ctx.method,
            uri = %ctx.original_uri,
            headers = ?headers,
        );

        DeviceResult::Continue
    }

    fn before_proxy(&self, ctx: &mut RequestCtx) -> DeviceResult {
        if !self.phase_enabled(LogPhase::Request) || !self.event_enabled(LogEvent::BeforeProxy) {
            return DeviceResult::Continue;
        }

        let headers = self.maybe_headers(&ctx.headers);

        emit!(
            self.level,
            event = "before_proxy",
            method = %ctx.method,
            uri = %ctx.original_uri,
            headers = ?headers,
        );

        DeviceResult::Continue
    }

    fn after_proxy(&self, ctx: &mut ResponseCtx) -> DeviceResult {
        if !self.phase_enabled(LogPhase::Response) || !self.event_enabled(LogEvent::AfterProxy) {
            return DeviceResult::Continue;
        }

        let headers = self.maybe_headers(&ctx.headers);

        emit!(
            self.level,
            event = "after_proxy",
            status = %ctx.status,
            headers = ?headers,
        );

        DeviceResult::Continue
    }

    fn on_response(&self, ctx: &mut ResponseCtx) -> DeviceResult {
        if !self.phase_enabled(LogPhase::Response) || !self.event_enabled(LogEvent::Response) {
            return DeviceResult::Continue;
        }

        let headers = self.maybe_headers(&ctx.headers);

        emit!(
            self.level,
            event = "response",
            status = %ctx.status,
            headers = ?headers,
        );

        DeviceResult::Continue
    }
}
