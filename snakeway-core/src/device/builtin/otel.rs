use crate::conf::types::OtelDeviceConfig;
use crate::ctx::{RequestCtx, RequestId, ResponseCtx};
use crate::device::core::errors::DeviceError;
use crate::device::core::{Device, result::DeviceResult};
use crate::enrichment::identity_field::IdentityField;
use crate::enrichment::user_agent::ClientIdentity;
use anyhow::Result;
use opentelemetry::{
    KeyValue,
    global,
    metrics::{Counter, Histogram},
};
use std::time::Instant;
use tracing::Span;
use tracing::field;

// ---------------------------------------------------------------------------
// Marker types stored in ctx.extensions
// ---------------------------------------------------------------------------

/// Stored in extensions at request start so we can compute duration at response time.
#[derive(Clone)]
pub(crate) struct OtelRequestStart(pub(crate) Instant);

/// Stored in extensions so the device can finalize the span on response.
#[derive(Clone)]
pub(crate) struct OtelRequestSpan(pub(crate) Span);

// ---------------------------------------------------------------------------
// Device
// ---------------------------------------------------------------------------

pub struct OtelDevice {
    identity_fields: Vec<IdentityField>,
    request_counter: Counter<u64>,
    request_duration: Histogram<f64>,
}

impl OtelDevice {
    pub fn from_config(cfg: OtelDeviceConfig) -> Result<Self> {
        let meter = global::meter("snakeway");

        let request_counter = meter
            .u64_counter("http.server.request.count")
            .with_description("Total HTTP requests processed")
            .build();

        let request_duration = meter
            .f64_histogram("http.server.request.duration")
            .with_description("HTTP request duration in seconds")
            .with_unit("s")
            .build();

        Ok(Self {
            identity_fields: cfg.identity_fields,
            request_counter,
            request_duration,
        })
    }

    /// Build identity KeyValue pairs from ClientIdentity + configured fields.
    fn identity_attributes(&self, identity: &ClientIdentity) -> Vec<KeyValue> {
        let geo = identity.geo.as_ref();
        let ua = identity.ua.as_ref();
        let mut attrs = Vec::with_capacity(self.identity_fields.len());

        for field in &self.identity_fields {
            match field {
                IdentityField::Country => {
                    if let Some(cc) = geo.and_then(|g| g.country_code.as_ref()) {
                        attrs.push(KeyValue::new("client.geo.country", cc.clone()));
                    }
                }
                IdentityField::Region => {
                    if let Some(r) = geo.and_then(|g| g.region.as_ref()) {
                        attrs.push(KeyValue::new("client.geo.region", r.clone()));
                    }
                }
                IdentityField::Asn => {
                    if let Some(asn) = geo.and_then(|g| g.asn) {
                        attrs.push(KeyValue::new("client.asn", asn as i64));
                    }
                }
                IdentityField::Aso => {
                    if let Some(aso) = geo.and_then(|g| g.aso.as_ref()) {
                        attrs.push(KeyValue::new("client.aso", aso.clone()));
                    }
                }
                IdentityField::ConnectionType => {
                    if let Some(ct) = geo.and_then(|g| g.connection_type.as_ref()) {
                        attrs.push(KeyValue::new("client.connection_type", ct.clone()));
                    }
                }
                IdentityField::Device => {
                    if let Some(ua) = ua {
                        attrs.push(KeyValue::new(
                            "client.device",
                            ua.device_type.as_str().to_string(),
                        ));
                    }
                }
                IdentityField::Bot => {
                    if let Some(ua) = ua {
                        attrs.push(KeyValue::new("client.is_bot", ua.is_bot));
                    }
                }
            }
        }

        attrs
    }

    /// Record identity fields onto a tracing Span.
    fn record_identity_on_span(&self, span: &Span, identity: &ClientIdentity) {
        let geo = identity.geo.as_ref();
        let ua = identity.ua.as_ref();

        for f in &self.identity_fields {
            match f {
                IdentityField::Country => {
                    if let Some(cc) = geo.and_then(|g| g.country_code.as_ref()) {
                        span.record("client.geo.country", cc.as_str());
                    }
                }
                IdentityField::Region => {
                    if let Some(r) = geo.and_then(|g| g.region.as_ref()) {
                        span.record("client.geo.region", r.as_str());
                    }
                }
                IdentityField::Asn => {
                    if let Some(asn) = geo.and_then(|g| g.asn) {
                        span.record("client.asn", asn as i64);
                    }
                }
                IdentityField::Aso => {
                    if let Some(aso) = geo.and_then(|g| g.aso.as_ref()) {
                        span.record("client.aso", aso.as_str());
                    }
                }
                IdentityField::ConnectionType => {
                    if let Some(ct) = geo.and_then(|g| g.connection_type.as_ref()) {
                        span.record("client.connection_type", ct.as_str());
                    }
                }
                IdentityField::Device => {
                    if let Some(ua) = ua {
                        span.record("client.device", ua.device_type.as_str());
                    }
                }
                IdentityField::Bot => {
                    if let Some(ua) = ua {
                        span.record("client.is_bot", ua.is_bot);
                    }
                }
            }
        }
    }
}

impl Device for OtelDevice {
    fn name(&self) -> &str {
        "OpenTelemetry"
    }

    fn on_request(&self, ctx: &mut RequestCtx) -> DeviceResult {
        // Record request start time for duration metric.
        ctx.extensions.insert(OtelRequestStart(Instant::now()));

        // Create a span. tracing-opentelemetry converts this into an OTel span.
        // All fields that may be set later must be declared as field::Empty here.
        let span = tracing::info_span!(
            "http.request",
            otel.kind = "server",
            "http.request.method" = ctx.method_str(),
            "url.path" = ctx.original_uri_path(),
            "http.response.status_code" = field::Empty,
            "request_id" = field::Empty,
            "error" = field::Empty,
            "client.geo.country" = field::Empty,
            "client.geo.region" = field::Empty,
            "client.asn" = field::Empty,
            "client.aso" = field::Empty,
            "client.connection_type" = field::Empty,
            "client.device" = field::Empty,
            "client.is_bot" = field::Empty,
        );

        // Fill request_id if available.
        if let Some(rid) = ctx.extensions.get::<RequestId>() {
            span.record("request_id", rid.0.as_str());
        }

        // Fill identity fields if IdentityDevice has already run.
        if let Some(identity) = ctx.extensions.get::<ClientIdentity>() {
            self.record_identity_on_span(&span, identity);
        }

        ctx.extensions.insert(OtelRequestSpan(span));
        DeviceResult::Continue
    }

    fn on_response(&self, ctx: &mut ResponseCtx) -> DeviceResult {
        let status = ctx.status.as_u16();

        // Record status on the span before it's finalized.
        if let Some(otel_span) = ctx.extensions.get::<OtelRequestSpan>() {
            otel_span.0.record("http.response.status_code", status as i64);
        }

        // Build metric attributes (low-cardinality only).
        let mut attrs = vec![KeyValue::new("http.response.status_code", status as i64)];

        // Add identity attributes to metrics.
        // We look up identity from the span extensions (identity was recorded on span at request time).
        // For metrics, pull identity out of the OtelRequestSpan if available.
        // Note: ClientIdentity is not in resp_ctx.extensions by default, but we added
        // it to the span already. For metrics, build attrs from what we have.
        // The identity was propagated via extensions from RequestCtx.
        if let Some(identity) = ctx.extensions.get::<ClientIdentity>() {
            attrs.extend(self.identity_attributes(identity));
        }

        self.request_counter.add(1, &attrs);

        // Record duration.
        if let Some(start) = ctx.extensions.get::<OtelRequestStart>() {
            let duration = start.0.elapsed().as_secs_f64();
            self.request_duration.record(duration, &attrs);
        }

        // The OtelRequestSpan and OtelRequestStart are dropped when ctx.extensions
        // is dropped at the end of response_filter(), which finalizes the OTel span.
        DeviceResult::Continue
    }

    fn on_error(&self, err: &DeviceError) {
        tracing::error!(
            event = "otel_device_error",
            fatal = err.fatal,
            message = %err.message,
        );
    }
}
