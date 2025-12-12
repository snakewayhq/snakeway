mod cli;
mod config;
mod ctx;
mod device;
mod logging;
mod proxy;
mod server;

use crate::logging::init_logging;
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

    /// Run the Snakeway proxy (default)
    Run {
        /// Path to the Snakeway config file
        #[arg(long, default_value = "config/snakeway.toml")]
        config: String,
    },
}

fn main() {
    init_logging();
    let cli = Cli::parse();

    match cli.command {
        // Explicit server run
        Some(Command::Run { config }) => {
            let cfg = SnakewayConfig::from_file(&config).expect("Failed to load Snakeway config");
            server::run(cfg).expect("Failed to start Snakeway server");
        }

        // Plugin tooling
        Some(Command::Plugin { cmd }) => {
            if let Err(e) = cli::plugin::run(cmd) {
                eprintln!("plugin error: {e}");
                std::process::exit(1);
            }
        }

        // No subcommand → behave exactly like before
        None => {
            let cfg = SnakewayConfig::from_file("config/snakeway.toml")
                .expect("Failed to load Snakeway config");
            server::run(cfg).expect("Failed to start Snakeway server");
        }
    }
}
