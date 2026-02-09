use crate::conf::{load_config, load_spec_files};
use clap::ValueEnum;
use serde::Serialize;
use std::path::PathBuf;

pub fn dump(
    path: PathBuf,
    format: ConfigDumpOutputFormat,
    repr: RepresentationFormat,
) -> anyhow::Result<()> {
    if matches!(repr, RepresentationFormat::Spec) {
        let cfg = load_spec_files(&path)?;
        match format {
            ConfigDumpOutputFormat::Json => dump_json(&cfg)?,
            ConfigDumpOutputFormat::Yaml => dump_yaml(&cfg)?,
        }
    } else if matches!(repr, RepresentationFormat::Runtime) {
        let cfg = load_config(&path)?;
        match format {
            ConfigDumpOutputFormat::Json => dump_json(&cfg.config)?,
            ConfigDumpOutputFormat::Yaml => dump_yaml(&cfg.config)?,
        }
    }

    Ok(())
}

fn dump_json<T: Serialize>(value: &T) -> anyhow::Result<()> {
    let s = serde_json::to_string_pretty(value)?;
    println!("{s}");
    Ok(())
}

fn dump_yaml<T: Serialize>(value: &T) -> anyhow::Result<()> {
    let s = serde_yaml::to_string(value)?;
    println!("{s}");
    Ok(())
}

#[derive(Clone, Debug, ValueEnum)]
pub enum RepresentationFormat {
    Spec,
    Runtime,
}

#[derive(Clone, Debug, ValueEnum)]
pub enum ConfigDumpOutputFormat {
    Json,
    Yaml,
}
