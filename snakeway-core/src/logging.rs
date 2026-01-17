use std::io::{self, IsTerminal};
use tracing_appender::rolling;
use tracing_subscriber::{EnvFilter, fmt};

/// Initialize the logging system with JSON formatting and environment-based filtering
///
/// This function sets up the logging infrastructure using tracing-subscriber:
/// - Uses environment variables for log level filtering (defaults to "info" if not set)
/// - Configures JSON output format for structured logging
/// - Flattens event fields for cleaner log output
pub fn init_normal_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    if let Ok(dir) = std::env::var("SNAKEWAY_LOG_DIR") {
        let appender = rolling::daily(dir, "snakeway.log");
        let (writer, guard) = tracing_appender::non_blocking(appender);

        fmt()
            .with_env_filter(filter)
            .json()
            .flatten_event(true)
            .with_writer(writer)
            .init();

        // Keep guard alive for the entire lifetime of the program.
        std::mem::forget(guard);
    } else {
        fmt()
            .with_env_filter(filter)
            .json()
            .flatten_event(true)
            .init();
    }
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
