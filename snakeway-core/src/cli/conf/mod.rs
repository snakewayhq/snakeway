mod check;
mod dump;
mod init;

pub use check::*;
use clap::Subcommand;
pub use dump::*;
pub use init::*;
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub enum ConfigCmd {
    /// Validate configuration and exit
    Check {
        /// Path to config directory
        #[arg(default_value = "config")]
        path: PathBuf,

        /// Suppresses all diagnostic
        #[arg(short, long)]
        quiet: bool,

        /// Emit machine readable diagnostics
        #[arg(short, long, default_value = "pretty", conflicts_with = "quiet")]
        format: ConfigCheckOutputFormat,
    },

    /// Print resolved configuration
    Dump {
        #[arg(default_value = "config")]
        path: PathBuf,

        #[arg(short, long, default_value = "dsl")]
        repr: RepresentationFormat,

        /// Output as JSON
        #[arg(long, conflicts_with = "yaml")]
        json: bool,

        /// Output as YAML
        #[arg(long)]
        yaml: bool,
    },

    /// Initialize a new config directory
    Init {
        /// Path to config directory
        #[arg(default_value = "config")]
        path: PathBuf,
    },
}
