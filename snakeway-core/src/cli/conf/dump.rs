use crate::conf::load_config;
use serde::Serialize;
use std::path::PathBuf;

pub fn dump(path: PathBuf, json: bool, yaml: bool) -> anyhow::Result<()> {
    let cfg = load_config(&path)?;

    if yaml {
        dump_yaml(&cfg.config)?;
    } else if json || !yaml {
        // default: json
        dump_json(&cfg.config)?;
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
