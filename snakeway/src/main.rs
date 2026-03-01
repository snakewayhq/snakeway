use clap::{Parser, Subcommand};
use snakeway_core::cli;
use snakeway_core::conf::load_config;
use snakeway_core::logging::{LogMode, default_log_mode, init_logging};
use snakeway_core::server;
use std::path::Path;
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
        cmd: cli::config::ConfigCmd,
    },

    /// Debug a WASM device in isolation
    WasmDevice {
        #[command(subcommand)]
        cmd: cli::wasm_device::WasmDeviceCmd,
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
        cmd: cli::route::RouteCmd,
    },

    /// Run the Snakeway proxy (default)
    Run {
        /// Path to the Snakeway config directory
        #[arg(long, default_value = "config")]
        config: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Config { cmd }) => match cmd {
            cli::config::ConfigCmd::Check {
                path,
                quiet,
                format,
            } => {
                if let Err(e) = cli::config::check(path, quiet, format) {
                    eprintln!("Invalid configuration\n\n{e}");
                    exit(1);
                }
            }
            cli::config::ConfigCmd::Dump { path, format, repr } => {
                if let Err(e) = cli::config::dump(path, format, repr) {
                    eprintln!("Failed to dump configuration: {e}");
                    exit(1);
                }
            }
            cli::config::ConfigCmd::Init { path, template } => {
                cli::config::init(path, template).expect("Failed to initialize config directory");
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
            cli::logs::run_logs(mode).expect("Failed to run logs command");
        }

        Some(Command::WasmDevice { cmd }) => {
            init_logging();

            if let Err(e) = cli::wasm_device::run(cmd) {
                eprintln!("WASM device error: {e}");
                exit(1);
            }
        }

        Some(Command::Route { cmd }) => {
            cli::route::run(cmd);
        }

        Some(Command::Reload { pid_file }) => {
            init_logging();

            if let Err(e) = cli::reload::run(&pid_file) {
                eprintln!("reload failed: {e}");
                exit(1);
            }
        }

        Some(Command::Run {
            config: config_path,
        }) => {
            run(&config_path);
        }

        None => {
            run("./config");
        }
    }
}

fn run(config_path: &str) {
    init_logging();

    let validated =
        load_config(Path::new(&config_path)).expect("Failed to load default Snakeway config");

    validated.validation_report.render_pretty();

    if validated.is_valid() {
        server::run(config_path, validated.config).expect("Failed to start Snakeway server");
    } else {
        exit(1);
    }
}
