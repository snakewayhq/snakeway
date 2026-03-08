use opentelemetry_sdk::trace::Tracer;
use std::sync::OnceLock;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Registry, fmt, layer::SubscriberExt};

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
fn init_normal_logging(tracer: Option<Tracer>) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    // The OpenTelemetry layer must be attached before fmt::Layer.
    // Otherwise its subscriber type becomes dependent on the fmt writer type
    // (stdout vs NonBlocking file writer), which causes nasty trait errors.
    let otel_layer = tracer.map(OpenTelemetryLayer::new);

    if let Ok(dir) = std::env::var("SNAKEWAY_LOG_DIR") {
        let appender = rolling::daily(dir, "snakeway.log");
        let (writer, guard) = tracing_appender::non_blocking(appender);

        let fmt_layer = fmt::layer().json().flatten_event(true).with_writer(writer);

        Registry::default()
            .with(filter)
            .with(otel_layer)
            .with(fmt_layer)
            .init();

        // Keep the non-blocking writer guard alive for the process lifetime.
        let _ = _LOG_GUARD.set(guard);
    } else {
        let fmt_layer = fmt::layer().json().flatten_event(true);

        Registry::default()
            .with(filter)
            .with(otel_layer)
            .with(fmt_layer)
            .init();
    }
}

pub fn init_logging(tracer: Option<Tracer>) {
    if std::env::var("TOKIO_CONSOLE").is_ok() {
        // Tokio console logging is specifically for interactive debugging and profiling.
        init_console_logging();
    } else {
        // Normal logging for production and non-interactive use.
        init_normal_logging(tracer);
    }
}

fn init_console_logging() {
    console_subscriber::init();
}

pub fn default_log_mode() -> LogMode {
    use std::io::IsTerminal;
    if std::io::stdout().is_terminal() {
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
