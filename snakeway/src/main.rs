mod config;
mod server;
mod proxy;
mod logging;

use clap::Parser;
use config::SnakewayConfig;
use crate::logging::init_logging;

#[derive(Parser, Debug)]
#[command(name = "snakeway", version, about = "Snakeway: Pingora-based HTTP proxy")]
struct Cli {
    /// Path to the Snakeway config file
    #[arg(long, default_value = "config/snakeway.toml")]
    config: String,
}

fn main() {
    init_logging();
    let cli = Cli::parse();
    let cfg = SnakewayConfig::from_file(&cli.config).expect("Failed to load Snakeway config");
    server::run(cfg).expect("Failed to start Snakeway server");
}
