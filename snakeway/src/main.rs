mod cli;

use crate::cli::logs::run_logs;
use clap::{Parser, Subcommand};
use snakeway_core::config::SnakewayConfig;
use snakeway_core::logging::{LogMode, default_log_mode, init_logging};
use snakeway_core::server;

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

    /// Run the Snakeway proxy (default)
    Run {
        /// Path to the Snakeway config file
        #[arg(long, default_value = "config/snakeway.toml")]
        config: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Logs { pretty, raw }) => {
            let mode = if raw {
                LogMode::Raw
            } else if pretty {
                LogMode::Pretty
            } else {
                default_log_mode()
            };
            run_logs(mode).expect("Failed to run logs command");
        }

        Some(Command::Plugin { cmd }) => {
            init_logging();

            if let Err(e) = cli::plugin::run(cmd) {
                eprintln!("plugin error: {e}");
                std::process::exit(1);
            }
        }

        Some(Command::Run { config }) => {
            init_logging();

            let cfg = SnakewayConfig::from_file(&config).expect("Failed to load Snakeway config");

            server::run(cfg).expect("Failed to start Snakeway server");
        }

        None => {
            init_logging();

            let cfg = SnakewayConfig::from_file("config/snakeway.toml")
                .expect("Failed to load Snakeway config");

            server::run(cfg).expect("Failed to start Snakeway server");
        }
    }
}
