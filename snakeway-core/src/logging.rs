use once_cell::sync::OnceCell;
use std::io::{self, IsTerminal};
use tracing_appender::rolling;
use tracing_subscriber::{
    EnvFilter, Registry, fmt, layer::SubscriberExt, reload, util::SubscriberInitExt,
};

type OTelLayer =
    tracing_opentelemetry::OpenTelemetryLayer<Registry, opentelemetry_sdk::trace::SdkTracer>;

static OTEL_HANDLE: OnceCell<reload::Handle<Option<OTelLayer>, Registry>> = OnceCell::new();

/// Initialize the logging system with JSON formatting and environment-based filtering.
///
/// Optionally sets up OTel tracing/metrics providers if OTEL_EXPORTER_OTLP_ENDPOINT
/// is set in the environment.
pub fn init_normal_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let fmt_layer = fmt::layer().json().flatten_event(true);

    let (otel_layer, handle) = reload::Layer::new(None);
    OTEL_HANDLE.set(handle).ok();

    let base = Registry::default();

    let subscriber = base.with(otel_layer).with(filter).with(fmt_layer);

    if let Ok(dir) = std::env::var("SNAKEWAY_LOG_DIR") {
        let appender = rolling::daily(dir, "snakeway.log");
        let (writer, guard) = tracing_appender::non_blocking(appender);

        let subscriber = subscriber.with(fmt::layer().json().with_writer(writer));

        tracing::subscriber::set_global_default(subscriber)
            .expect("failed to set tracing subscriber");

        std::mem::forget(guard);
    } else {
        tracing::subscriber::set_global_default(subscriber)
            .expect("failed to set tracing subscriber");
    }
}

pub fn enable_otel_from_env() {
    let endpoint = match std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT") {
        Ok(v) => v,
        Err(_) => return,
    };

    let service_name =
        std::env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| "snakeway".to_string());

    let tracer = match init_otel_providers(&endpoint, &service_name) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Failed to initialize OTel providers: {e}");
            return;
        }
    };

    let layer = tracing_opentelemetry::layer().with_tracer(tracer);

    if let Some(handle) = OTEL_HANDLE.get() {
        let _ = handle.modify(|slot| {
            *slot = Some(layer);
        });
    }
}

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
        init_console_logging();
    } else {
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
