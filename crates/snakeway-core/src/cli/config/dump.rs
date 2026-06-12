use crate::cli::config::hcl::to_hcl_string;
use clap::ValueEnum;
use serde::Serialize;
use snakeway_conf::{load_config, load_spec_files};
use std::path::PathBuf;

pub(crate) fn dump(
    path: PathBuf,
    format: ConfigDumpOutputFormat,
    repr: RepresentationFormat,
) -> anyhow::Result<()> {
    if matches!(repr, RepresentationFormat::Spec) {
        let (_sources, _report, server, devices, ingresses) = load_spec_files(&path)?;
        let cfg = (server, devices, ingresses);
        match format {
            ConfigDumpOutputFormat::Json => dump_json(&cfg)?,
            ConfigDumpOutputFormat::Yaml => dump_yaml(&cfg)?,
            ConfigDumpOutputFormat::Hcl => dump_hcl(&cfg)?,
        }
    } else if matches!(repr, RepresentationFormat::Runtime) {
        let cfg = load_config(&path)?.config;
        match format {
            ConfigDumpOutputFormat::Json => dump_json(&cfg)?,
            ConfigDumpOutputFormat::Yaml => dump_yaml(&cfg)?,
            ConfigDumpOutputFormat::Hcl => dump_hcl(&cfg)?,
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

fn dump_hcl<T: Serialize>(value: &T) -> anyhow::Result<()> {
    let s = to_hcl_string(value)?;
    println!("{s}");
    Ok(())
}

#[derive(Clone, Debug, ValueEnum)]
pub(crate) enum RepresentationFormat {
    Spec,
    Runtime,
}

#[derive(Clone, Debug, ValueEnum)]
pub(crate) enum ConfigDumpOutputFormat {
    Hcl,
    Json,
    Yaml,
}
