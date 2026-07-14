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
    match repr {
        // The spec as written: nested blocks the source omitted stay omitted.
        RepresentationFormat::Spec => {
            let (_sources, _report, server, devices, ingresses) = load_spec_files(&path)?;
            dump_value(&(server, devices, ingresses), format)
        }
        // The spec with defaulted nested blocks filled in, so an operator can
        // see the values the runtime will apply for blocks they did not write.
        RepresentationFormat::PopulatedSpec => {
            let (_sources, _report, server, devices, ingresses) = load_spec_files(&path)?;
            dump_value(&(server.populated(), devices, ingresses), format)
        }
        RepresentationFormat::Runtime => {
            let cfg = load_config(&path)?.config;
            dump_value(&cfg, format)
        }
    }
}

fn dump_value<T: Serialize>(value: &T, format: ConfigDumpOutputFormat) -> anyhow::Result<()> {
    match format {
        ConfigDumpOutputFormat::Json => dump_json(value),
        ConfigDumpOutputFormat::Yaml => dump_yaml(value),
        ConfigDumpOutputFormat::Hcl => dump_hcl(value),
    }
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
    /// The parsed spec exactly as written, omitting blocks the source did not set.
    Spec,
    /// The parsed spec with defaulted nested blocks filled in.
    PopulatedSpec,
    Runtime,
}

#[derive(Clone, Debug, ValueEnum)]
pub(crate) enum ConfigDumpOutputFormat {
    Hcl,
    Json,
    Yaml,
}
