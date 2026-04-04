use crate::control_plane::observability::TelemetryProviders;
use opentelemetry::trace::TracerProvider;
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use std::sync::OnceLock;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer, Registry, fmt, layer::SubscriberExt};

/// Holds the non-blocking writer guard for the process lifetime.
///
/// The guard must remain alive as long as log output should be flushed to the
/// file. Storing it in a static prevents the background writer thread from
/// being torn down prematurely, without leaking memory via `mem::forget`.
static _LOG_GUARD: OnceLock<WorkerGuard> = OnceLock::new();

/// Initialize the logging system with JSON formatting and environment-based filtering
///
/// This function sets up the logging infrastructure using tracing-subscriber:
/// - Uses environment variables for log level filtering (defaults to "info" if not set)
/// - Configures JSON output format for structured logging
/// - Flattens event fields for cleaner log output
fn init_normal_logging(maybe_telemetry_providers: Option<TelemetryProviders>) {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    // The OpenTelemetry layers must be attached before fmt::Layer.
    // Otherwise, their subscriber types become dependent on the fmt writer type
    // (stdout vs. NonBlocking file writer), which causes nasty trait errors.
    let otel_filter = EnvFilter::new("info")
        .add_directive("pingora=off".parse().unwrap())
        .add_directive("tonic=off".parse().unwrap())
        .add_directive("h2=off".parse().unwrap())
        .add_directive("reqwest=off".parse().unwrap());
    let (tracer_provider, logger_provider) = match maybe_telemetry_providers {
        None => (None, None),
        Some(providers) => (
            Some(providers.tracer_provider),
            Some(providers.logger_provider),
        ),
    };
    let tracer_layer = tracer_provider.map(|p| OpenTelemetryLayer::new(p.tracer("snakeway")));
    let logger_layer =
        logger_provider.map(|p| OpenTelemetryTracingBridge::new(&p).with_filter(otel_filter));

    if let Ok(dir) = std::env::var("SNAKEWAY_LOG_DIR") {
        let appender = rolling::daily(dir, "snakeway.log");
        let (writer, guard) = tracing_appender::non_blocking(appender);

        let fmt_layer = fmt::layer().json().flatten_event(true).with_writer(writer);

        Registry::default()
            .with(env_filter)
            .with(tracer_layer)
            .with(logger_layer)
            .with(fmt_layer)
            .init();

        // Keep the non-blocking writer guard alive for the process lifetime.
        let _ = _LOG_GUARD.set(guard);
    } else {
        let fmt_layer = fmt::layer().json().flatten_event(true);

        Registry::default()
            .with(env_filter)
            .with(tracer_layer)
            .with(logger_layer)
            .with(fmt_layer)
            .init();
    }
}

pub(crate) fn init_logging(telemetry_providers: Option<TelemetryProviders>) {
    if std::env::var("TOKIO_CONSOLE").is_ok() {
        // Tokio console logging is specifically for interactive debugging and profiling.
        console_subscriber::init();
    } else {
        // Normal logging for production and non-interactive use.
        init_normal_logging(telemetry_providers);
    }
}
