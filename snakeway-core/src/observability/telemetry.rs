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
use tracing::info;

/// Global tracer provider so we can flush spans on shutdown.
static TRACER_PROVIDER: OnceCell<SdkTracerProvider> = OnceCell::new();

/// Initialize OpenTelemetry tracing if configured.
///
/// This must be called **after configuration is loaded**
/// but **before Pingora worker threads start**.
pub fn init_telemetry(config: &RuntimeConfig) -> Option<Tracer> {
    let Some(obs) = &config.server.observability else {
        return None;
    };

    let Some(otel) = &obs.otel else {
        return None;
    };

    if !otel.enable {
        return None;
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
        .expect("failed to create OTLP exporter");

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

    // Store globally so we can flush on shutdown
    TRACER_PROVIDER
        .set(tracer_provider.clone())
        .expect("tracer provider already initialized");

    global::set_tracer_provider(tracer_provider);

    info!("OpenTelemetry tracing initialized");

    Some(tracer)
}

/// Shutdown telemetry and flush remaining spans.
///
/// Optional but recommended for graceful shutdown.
pub fn shutdown() {
    if let Some(provider) = TRACER_PROVIDER.get() {
        let _ = provider.shutdown();
    }
}
