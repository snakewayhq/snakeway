use once_cell::sync::OnceCell;
use opentelemetry::{KeyValue, global};
use opentelemetry_otlp::{LogExporter, MetricExporter, SpanExporter, WithExportConfig};
use opentelemetry_sdk::logs::SdkLoggerProvider;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::{
    Resource,
    trace::{Sampler, SdkTracerProvider},
};
use snakeway_conf::types::{RuntimeConfig, SamplingTypeConfig};
use tracing::{info, warn};

/// Global tracer provider so we can flush spans on shutdown.
static TRACER_PROVIDER: OnceCell<SdkTracerProvider> = OnceCell::new();

/// Global metrics provider so we can flush metrics on shutdown.
static METER_PROVIDER: OnceCell<SdkMeterProvider> = OnceCell::new();

/// Initialize OpenTelemetry tracing if configured.
///
/// This must be called **after configuration is loaded**
/// but **before Pingora worker threads start**.
///
/// Returns `Ok(Some(tracer))` when tracing is enabled and initialized,
/// `Ok(None)` when tracing is disabled or not configured, and
/// `Err(...)` when the exporter fails to build.
pub(crate) async fn init_telemetry(
    config: &RuntimeConfig,
) -> Result<Option<TelemetryProviders>, Box<dyn std::error::Error>> {
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
    // Exporters
    //-------------------------------------------------------------------------

    let span_exporter = SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()
        .map_err(|e| format!("failed to create OTLP span exporter: {e}"))?;

    let log_exporter = LogExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()
        .map_err(|e| format!("failed to create OTLP log exporter: {e}"))?;

    let metric_exporter = MetricExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()
        .map_err(|e| format!("failed to create OTLP metric exporter: {e}"))?;

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
    // Providers
    //-------------------------------------------------------------------------

    // Spans
    let tracer_provider = SdkTracerProvider::builder()
        .with_sampler(sampler)
        .with_batch_exporter(span_exporter)
        .with_resource(resource.clone())
        .build();

    // Logs
    let logger_provider = SdkLoggerProvider::builder()
        .with_resource(resource.clone())
        .with_batch_exporter(log_exporter)
        .build();

    // Metrics
    let meter_provider = SdkMeterProvider::builder()
        .with_resource(resource.clone())
        .with_periodic_exporter(metric_exporter)
        .build();

    let is_tracer_initialized = TRACER_PROVIDER.set(tracer_provider.clone()).is_err();
    let is_meter_initialized = METER_PROVIDER.set(meter_provider.clone()).is_err();

    if is_tracer_initialized {
        warn!("tracer provider was already initialized; skipping re-initialization");
    } else {
        // Allows for flushing spans on shutdown.
        global::set_tracer_provider(tracer_provider.clone());
    }

    if is_meter_initialized {
        warn!("meter provider was already initialized; skipping re-initialization");
    } else {
        // Allows for flushing metrics on shutdown.
        global::set_meter_provider(meter_provider.clone());
    }

    info!("OpenTelemetry support initialized");

    Ok(Some(TelemetryProviders {
        tracer_provider,
        logger_provider,
    }))
}

pub(crate) struct TelemetryProviders {
    pub(crate) tracer_provider: SdkTracerProvider,
    pub(crate) logger_provider: SdkLoggerProvider,
}

/// Shutdown telemetry and flush remaining spans.
pub(crate) fn shutdown() {
    if let Some(provider) = TRACER_PROVIDER.get() {
        let _ = provider.shutdown();
    }

    if let Some(provider) = METER_PROVIDER.get() {
        let _ = provider.shutdown();
    }
}
