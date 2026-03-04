use std::io::{self, IsTerminal};
use tracing_appender::rolling;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize the logging system with JSON formatting and environment-based filtering.
///
/// Optionally sets up OTel tracing/metrics providers if OTEL_EXPORTER_OTLP_ENDPOINT
/// is set in the environment.
pub fn init_normal_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let fmt_layer = fmt::layer().json().flatten_event(true);

    // Try to initialize OTel providers from environment variables.
    let otel_tracer = try_init_otel_providers();

    if let Ok(dir) = std::env::var("SNAKEWAY_LOG_DIR") {
        let appender = rolling::daily(dir, "snakeway.log");
        let (writer, guard) = tracing_appender::non_blocking(appender);

        if let Some(tracer) = otel_tracer {
            tracing_subscriber::registry()
                .with(filter)
                .with(fmt_layer.with_writer(writer))
                .with(tracing_opentelemetry::layer().with_tracer(tracer))
                .init();
        } else {
            tracing_subscriber::registry()
                .with(filter)
                .with(fmt_layer.with_writer(writer))
                .init();
        }

        // Keep guard alive for the entire lifetime of the program.
        std::mem::forget(guard);
    } else if let Some(tracer) = otel_tracer {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt_layer)
            .with(tracing_opentelemetry::layer().with_tracer(tracer))
            .init();
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt_layer)
            .init();
    }
}

/// Attempt to initialize OTel TracerProvider and MeterProvider from env vars.
/// Returns the tracer if setup succeeded, or None if OTEL_EXPORTER_OTLP_ENDPOINT
/// is not set or setup fails.
fn try_init_otel_providers() -> Option<opentelemetry_sdk::trace::SdkTracer> {
    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok()?;
    let service_name = std::env::var("OTEL_SERVICE_NAME")
        .unwrap_or_else(|_| "snakeway".to_string());

    match init_otel_providers(&endpoint, &service_name) {
        Ok(tracer) => Some(tracer),
        Err(err) => {
            // Can't use tracing here (not yet initialized), so use eprintln.
            eprintln!("Failed to initialize OTel providers: {err}");
            None
        }
    }
}

/// Initialize OTel TracerProvider and MeterProvider, returning a Tracer.
/// Sets the global providers as a side effect.
pub fn init_otel_providers(
    endpoint: &str,
    service_name: &str,
) -> anyhow::Result<opentelemetry_sdk::trace::SdkTracer> {
    use opentelemetry::KeyValue;
    use opentelemetry::trace::TracerProvider as _;
    use opentelemetry_otlp::WithExportConfig;
    use opentelemetry_sdk::trace::SdkTracerProvider;

    let resource = opentelemetry_sdk::Resource::builder_empty()
        .with_attribute(KeyValue::new("service.name", service_name.to_string()))
        .build();

    // --- Trace provider ---
    let span_exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()?;

    let tracer_provider = SdkTracerProvider::builder()
        .with_resource(resource.clone())
        .with_batch_exporter(span_exporter)
        .build();

    let tracer = tracer_provider.tracer("snakeway");
    opentelemetry::global::set_tracer_provider(tracer_provider);

    // --- Metrics provider ---
    let metric_exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()?;

    let reader = opentelemetry_sdk::metrics::PeriodicReader::builder(metric_exporter).build();

    let meter_provider = opentelemetry_sdk::metrics::SdkMeterProvider::builder()
        .with_reader(reader)
        .with_resource(resource)
        .build();

    opentelemetry::global::set_meter_provider(meter_provider);

    Ok(tracer)
}

pub fn init_logging() {
    if std::env::var("TOKIO_CONSOLE").is_ok() {
        // Tokio console logging is specifically for interactive debugging and profiling.
        init_console_logging();
    } else {
        // Normal logging for production and non-interactive use.
        init_normal_logging();
    }
}

fn init_console_logging() {
    console_subscriber::init();
}

pub fn default_log_mode() -> LogMode {
    if io::stdout().is_terminal() {
        LogMode::Pretty
    } else {
        LogMode::Raw
    }
}

#[derive(Debug, Clone, Copy)]
pub enum LogMode {
    Raw,
    Pretty,
    Stats,
}
