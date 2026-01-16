use std::io::{self, IsTerminal};
use tracing_subscriber::{EnvFilter, fmt};

/// Initialize the logging system with JSON formatting and environment-based filtering
///
/// This function sets up the logging infrastructure using tracing-subscriber:
/// - Uses environment variables for log level filtering (defaults to "info" if not set)
/// - Configures JSON output format for structured logging
/// - Flattens event fields for cleaner log output
pub fn init_normal_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    fmt()
        .with_env_filter(filter)
        .json()
        .flatten_event(true)
        .init();
}

pub fn init_logging() {
    // If tokio-console is enabled, DO NOT install your normal subscriber
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
