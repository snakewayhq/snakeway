use crate::conf::load_config;
use anyhow::Result;
use clap::Subcommand;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub enum ConfigCmd {
    /// Validate configuration and exit
    Check {
        /// Path to config directory
        #[arg(default_value = "config")]
        path: PathBuf,
    },

    /// Print resolved configuration
    Dump {
        #[arg(default_value = "config")]
        path: PathBuf,

        /// Output as JSON
        #[arg(long, conflicts_with = "yaml")]
        json: bool,

        /// Output as YAML
        #[arg(long)]
        yaml: bool,
    },
}

pub fn check(path: PathBuf) -> Result<()> {
    let cfg = load_config(&path)?;

    println!("✔ Config loaded successfully");
    println!("✔ {} routes", cfg.routes.len());
    println!("✔ {} services", cfg.services.len());
    println!(
        "✔ {} upstreams",
        cfg.services
            .values()
            .map(|s| s.upstream.len())
            .sum::<usize>()
    );
    println!(
        "✔ {} devices enabled",
        cfg.devices.iter().filter(|d| d.enabled).count()
    );

    Ok(())
}

pub fn dump(path: PathBuf, json: bool, yaml: bool) -> Result<()> {
    let cfg = load_config(&path)?;

    if yaml {
        dump_yaml(&cfg)?;
    } else if json || !yaml {
        // default: json
        dump_json(&cfg)?;
    }

    Ok(())
}

fn dump_json<T: Serialize>(value: &T) -> Result<()> {
    let s = serde_json::to_string_pretty(value)?;
    println!("{s}");
    Ok(())
}

fn dump_yaml<T: Serialize>(value: &T) -> Result<()> {
    let s = serde_yaml::to_string(value)?;
    println!("{s}");
    Ok(())
}
