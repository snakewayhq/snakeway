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

        /// Specify the output format
        #[arg(
            short,
            long,
            value_enum,
            default_value = "pretty",
            conflicts_with = "quiet"
        )]
        format: ConfigCheckOutputFormat,
    },

    /// Print resolved configuration
    Dump {
        #[arg(default_value = "config")]
        path: PathBuf,

        /// Specify the output representation: spec -> config files and runtime -> internal state
        #[arg(short, long, value_enum, default_value = "spec")]
        repr: RepresentationFormat,

        /// Specify the output format
        #[arg(short, long, value_enum, default_value = "json")]
        format: ConfigDumpOutputFormat,
    },

    /// Initialize a new config directory
    Init {
        /// Where the new config directory should be created
        #[arg(default_value = "config")]
        path: PathBuf,

        /// Specify the template to use
        #[arg(short, long, value_enum, default_value = "default")]
        template: ConfigInitTemplate,
    },
}
