use clap::{Parser, Subcommand};
use snakeway_core::cli;
use snakeway_core::conf::load_config;
use snakeway_core::logging::{LogMode, default_log_mode, init_logging};
use snakeway_core::server;
use std::path::Path;

#[derive(Parser, Debug)]
#[command(
    name = "snakeway",
    version,
    about = "Snakeway: Pingora-based HTTP proxy"
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
        cmd: cli::conf::ConfigCmd,
    },

    /// WASM plugin tooling
    Plugin {
        #[command(subcommand)]
        cmd: cli::plugin::PluginCmd,
    },

    Logs {
        #[arg(long)]
        pretty: bool,

        #[arg(long)]
        raw: bool,
    },

    /// Reload a running Snakeway instance (SIGHUP)
    Reload {
        /// Path to pid file
        #[arg(long, default_value = "/tmp/snakeway.pid")]
        pid_file: String,
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
            cli::conf::ConfigCmd::Check { path, plain } => {
                if let Err(e) = cli::conf::check(path, plain) {
                    eprintln!("Invalid configuration\n\n{e}");
                    std::process::exit(1);
                }
            }
            cli::conf::ConfigCmd::Dump { path, json, yaml } => {
                if let Err(e) = cli::conf::dump(path, json, yaml) {
                    eprintln!("Failed to dump configuration: {e}");
                    std::process::exit(1);
                }
            }
            cli::conf::ConfigCmd::Init { path } => {
                cli::conf::init(path).expect("Failed to initialize config directory");
            }
        },

        Some(Command::Logs { pretty, raw }) => {
            let mode = if raw {
                LogMode::Raw
            } else if pretty {
                LogMode::Pretty
            } else {
                default_log_mode()
            };
            cli::logs::run_logs(mode).expect("Failed to run logs command");
        }

        Some(Command::Plugin { cmd }) => {
            init_logging();

            if let Err(e) = cli::plugin::run(cmd) {
                eprintln!("plugin error: {e}");
                std::process::exit(1);
            }
        }

        Some(Command::Reload { pid_file }) => {
            init_logging();

            if let Err(e) = cli::reload::run(&pid_file) {
                eprintln!("reload failed: {e}");
                std::process::exit(1);
            }
        }

        Some(Command::Run {
            config: config_path,
        }) => {
            init_logging();

            let cfg = load_config(Path::new(&config_path)).expect("Failed to load Snakeway config");
            server::run(config_path, cfg).expect("Failed to start Snakeway server");
        }

        None => {
            init_logging();

            let config_path = "config".to_string();
            let cfg = load_config(Path::new(&config_path))
                .expect("Failed to load default Snakeway config");
            server::run(config_path, cfg).expect("Failed to start Snakeway server");
        }
    }
}
