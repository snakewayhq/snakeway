use crate::cli::config::{ConfigCmd, check, dump, init};
use crate::cli::logs::run_logs;
use crate::cli::route::RouteCmd;
use crate::cli::wasm_device::WasmDeviceCmd;
use crate::cli::{reload, route, wasm_device};
use crate::control_plane::observability::init_logging;
use crate::server;
use clap::{Parser, Subcommand};
use std::process::exit;

#[derive(Parser, Debug)]
#[command(
    name = "snakeway",
    version,
    about = "Snakeway: A HTTP proxy built with Rust"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Inspect configuration
    Config {
        #[command(subcommand)]
        cmd: ConfigCmd,
    },

    /// Debug a WASM device in isolation
    WasmDevice {
        #[command(subcommand)]
        cmd: WasmDeviceCmd,
    },

    /// Format logs from standard out
    Logs {
        #[arg(long)]
        pretty: bool,

        #[arg(long)]
        raw: bool,

        #[arg(long)]
        stats: bool,
    },

    /// Reload a running Snakeway instance (SIGHUP)
    Reload {
        /// Path to pid file
        #[arg(long, default_value = "/tmp/snakeway.pid")]
        pid_file: String,
    },

    /// Route debugging tools
    Route {
        #[command(subcommand)]
        cmd: RouteCmd,
    },

    /// Run the Snakeway proxy (default)
    Run {
        /// Path to the Snakeway config directory
        #[arg(long, default_value = "config", env = "SNAKEWAY_CONFIG")]
        config: String,

        /// Start in upgrade mode: receive listener FDs from a running instance
        /// instead of binding fresh sockets. Used during zero-drop upgrades.
        #[arg(long)]
        upgrade: bool,

        /// Validate config and exit without starting the server.
        /// Useful for pre-checking before a zero-drop upgrade.
        #[arg(long)]
        test: bool,
    },
}

pub fn run() {
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Config { cmd }) => match cmd {
            ConfigCmd::Check {
                path,
                quiet,
                format,
            } => {
                if let Err(e) = check(path, quiet, format) {
                    eprintln!("Invalid configuration\n\n{e}");
                    exit(1);
                }
            }
            ConfigCmd::Dump { path, format, repr } => {
                if let Err(e) = dump(path, format, repr) {
                    eprintln!("Failed to dump configuration: {e}");
                    exit(1);
                }
            }
            ConfigCmd::Init { path, template } => {
                init(path, template).expect("Failed to initialize config directory");
            }
        },

        Some(Command::Logs { pretty, raw, stats }) => {
            let mode = if raw {
                LogMode::Raw
            } else if pretty {
                LogMode::Pretty
            } else if stats {
                LogMode::Stats
            } else {
                default_log_mode()
            };
            run_logs(mode).expect("Failed to run logs command");
        }

        Some(Command::WasmDevice { cmd }) => {
            init_logging(None);

            if let Err(e) = wasm_device::run(cmd) {
                eprintln!("WASM device error: {e}");
                exit(1);
            }
        }

        Some(Command::Route { cmd }) => {
            route::run(cmd);
        }

        Some(Command::Reload { pid_file }) => {
            init_logging(None);

            if let Err(e) = reload::run(&pid_file) {
                eprintln!("reload failed: {e}");
                exit(1);
            }
        }

        Some(Command::Run {
            config: config_path,
            upgrade,
            test,
        }) => {
            server::start_server(&config_path, upgrade, test);
        }

        None => {
            let config_path =
                std::env::var(super::SNAKEWAY_CONFIG_ENV).unwrap_or_else(|_| "config".to_string());
            server::start_server(&config_path, false, false);
        }
    }
}

pub(crate) fn default_log_mode() -> LogMode {
    use std::io::IsTerminal;
    if std::io::stdout().is_terminal() {
        LogMode::Pretty
    } else {
        LogMode::Raw
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum LogMode {
    Raw,
    Pretty,
    Stats,
}
