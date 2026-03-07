use crate::conf::RuntimeConfig;
use opentelemetry::trace::TracerProvider;
use opentelemetry::{KeyValue, global};
use opentelemetry_otlp::{SpanExporter, WithExportConfig};
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_sdk::{Resource, trace::Tracer};
use tracing::info;

pub fn init(config: &RuntimeConfig) -> Option<Tracer> {
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

    // Build OTLP exporter
    let exporter = SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()
        .expect("failed to create OTLP exporter");

    // Build tracer provider
    let tracer_provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(
            Resource::builder()
                .with_attributes([KeyValue::new("service.name", service_name.to_string())])
                .build(),
        )
        .build();

    let tracer = tracer_provider.tracer("snakeway");

    global::set_tracer_provider(tracer_provider);

    info!("OpenTelemetry tracing initialized");

    Some(tracer)
}
