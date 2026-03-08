use crate::conf::RuntimeConfig;
use crate::conf::types::SamplingTypeConfig;
use once_cell::sync::OnceCell;
use opentelemetry::trace::TracerProvider;
use opentelemetry::{KeyValue, global};
use opentelemetry_otlp::{SpanExporter, WithExportConfig};
use opentelemetry_sdk::{
    Resource,
    trace::{Sampler, SdkTracerProvider, Tracer},
};
use tracing::{info, warn};

/// Global tracer provider so we can flush spans on shutdown.
static TRACER_PROVIDER: OnceCell<SdkTracerProvider> = OnceCell::new();

/// Initialize OpenTelemetry tracing if configured.
///
/// This must be called **after configuration is loaded**
/// but **before Pingora worker threads start**.
///
/// Returns `Ok(Some(tracer))` when tracing is enabled and initialized,
/// `Ok(None)` when tracing is disabled or not configured, and
/// `Err(...)` when the exporter fails to build.
pub fn init_telemetry(
    config: &RuntimeConfig,
) -> Result<Option<Tracer>, Box<dyn std::error::Error>> {
    let Some(obs) = &config.server.observability else {
        return Ok(None);
    };

    let Some(otel) = &obs.otel else {
        return Ok(None);
    };

    if !otel.enable {
        return Ok(None);
    }

    let endpoint = &otel.endpoint;
    let service_name = &otel.service_name;

    info!(
        endpoint = %endpoint,
        service_name = %service_name,
        "initializing OpenTelemetry exporter"
    );

    //-------------------------------------------------------------------------
    // Exporter
    //-------------------------------------------------------------------------

    let exporter = SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()
        .map_err(|e| format!("failed to create OTLP exporter: {e}"))?;

    //-------------------------------------------------------------------------
    // Sampling
    //-------------------------------------------------------------------------

    let sampler = match otel.sampling {
        SamplingTypeConfig::ParentBased => Sampler::ParentBased(Box::new(Sampler::AlwaysOn)),
    };

    //-------------------------------------------------------------------------
    // Resource metadata
    //-------------------------------------------------------------------------

    let resource = Resource::builder()
        .with_attributes([
            KeyValue::new("service.name", service_name.to_string()),
            KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
            KeyValue::new(
                "service.instance.id",
                hostname::get()
                    .ok()
                    .and_then(|h| h.into_string().ok())
                    .unwrap_or_else(|| "unknown".into()),
            ),
        ])
        .build();

    //-------------------------------------------------------------------------
    // Tracer provider
    //-------------------------------------------------------------------------

    let tracer_provider = SdkTracerProvider::builder()
        .with_sampler(sampler)
        .with_batch_exporter(exporter)
        .with_resource(resource)
        .build();

    let tracer = tracer_provider.tracer("snakeway");

    // Store globally so we can flush on shutdown.
    // The clone is a reference-counted handle; both instances share the same provider.
    if TRACER_PROVIDER.set(tracer_provider.clone()).is_err() {
        warn!("tracer provider was already initialized; skipping re-initialization");
        return Ok(None);
    }

    global::set_tracer_provider(tracer_provider);

    info!("OpenTelemetry tracing initialized");

    Ok(Some(tracer))
}

/// Shutdown telemetry and flush remaining spans.
///
/// Optional but recommended for graceful shutdown.
pub fn shutdown() {
    if let Some(provider) = TRACER_PROVIDER.get() {
        let _ = provider.shutdown();
    }
}
