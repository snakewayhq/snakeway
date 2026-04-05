use std::sync::OnceLock;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Registry, fmt, layer::SubscriberExt};

/// Holds the non-blocking writer guard for the process lifetime.
///
/// The guard must remain alive as long as log output should be flushed to the
/// file. Storing it in a static prevents the background writer thread from
/// being torn down prematurely, without leaking memory via `mem::forget`.
static _LOG_GUARD: OnceLock<WorkerGuard> = OnceLock::new();

/// OTel logging
#[cfg(feature = "otel")]
fn init_normal_logging(maybe_telemetry_providers: Option<super::TelemetryProviders>) {
    use opentelemetry::trace::TracerProvider;
    use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
    use tracing_opentelemetry::OpenTelemetryLayer;
    use tracing_subscriber::Layer;

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let otel_filter = EnvFilter::new("info")
        .add_directive("pingora=off".parse().expect("valid directive"))
        .add_directive("hyper=off".parse().expect("valid directive"))
        .add_directive("tower=off".parse().expect("valid directive"))
        .add_directive("tonic=off".parse().expect("valid directive"))
        .add_directive("h2=off".parse().expect("valid directive"))
        .add_directive("reqwest=off".parse().expect("valid directive"));

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

#[cfg(feature = "otel")]
pub(crate) fn init_logging(telemetry_providers: Option<super::TelemetryProviders>) {
    if std::env::var("TOKIO_CONSOLE").is_ok() {
        console_subscriber::init();
    } else {
        init_normal_logging(telemetry_providers);
    }
}

/// Non-OTel logging (structured JSON only)
#[cfg(not(feature = "otel"))]
fn init_normal_logging() {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    if let Ok(dir) = std::env::var("SNAKEWAY_LOG_DIR") {
        let appender = rolling::daily(dir, "snakeway.log");
        let (writer, guard) = tracing_appender::non_blocking(appender);
        let fmt_layer = fmt::layer().json().flatten_event(true).with_writer(writer);

        Registry::default().with(env_filter).with(fmt_layer).init();

        let _ = _LOG_GUARD.set(guard);
    } else {
        let fmt_layer = fmt::layer().json().flatten_event(true);

        Registry::default().with(env_filter).with(fmt_layer).init();
    }
}

#[cfg(not(feature = "otel"))]
pub(crate) fn init_logging() {
    if std::env::var("TOKIO_CONSOLE").is_ok() {
        console_subscriber::init();
    } else {
        init_normal_logging();
    }
}
