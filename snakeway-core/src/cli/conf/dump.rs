use crate::conf::{load_config, load_dsl_config};
use serde::Serialize;
use std::path::PathBuf;
use std::str::FromStr;

pub fn dump(
    path: PathBuf,
    json: bool,
    yaml: bool,
    repr: RepresentationFormat,
) -> anyhow::Result<()> {
    if matches!(repr, RepresentationFormat::Dsl) {
        let cfg = load_dsl_config(&path)?;
        if yaml {
            dump_yaml(&cfg)?;
        } else if json || !yaml {
            dump_json(&cfg)?;
        }
    } else if matches!(repr, RepresentationFormat::Runtime) {
        let cfg = load_config(&path)?;
        if yaml {
            dump_yaml(&cfg.config)?;
        } else if json || !yaml {
            dump_json(&cfg.config)?;
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

#[derive(Clone, Debug)]
pub enum RepresentationFormat {
    Dsl,
    Runtime,
}

impl FromStr for RepresentationFormat {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "dsl" => Ok(Self::Dsl),
            "runtime" => Ok(Self::Runtime),
            _ => Err(anyhow::anyhow!("invalid output format: {}", s)),
        }
    }
}
