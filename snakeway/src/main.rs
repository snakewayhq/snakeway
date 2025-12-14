mod cli;
mod config;
mod ctx;
mod device;
mod logging;
mod proxy;
mod server;

use crate::cli::logs::run_logs;
use crate::logging::{init_logging, LogMode};
use clap::{Parser, Subcommand};
use config::SnakewayConfig;

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
        Some(Command::Logs {
            pretty,
            raw,
        }) => {
            let mode = if raw {
                LogMode::Raw
            } else if pretty {
                LogMode::Pretty
            } else {
                logging::default_log_mode()
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
