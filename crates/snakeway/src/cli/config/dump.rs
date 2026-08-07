use clap::ValueEnum;
use confval::format::hcl::emit_hcl;
use confval::format::{Field, Fields, ToFields, Walk};
use hcl::format::Formatter;
use hcl::ser::Serializer;
use serde::Serialize;
use snakeway_conf::{load_config, load_spec_files};
use std::path::PathBuf;

pub(crate) fn dump(
    path: PathBuf,
    format: ConfigDumpOutputFormat,
    repr: RepresentationFormat,
) -> anyhow::Result<()> {
    match repr {
        // The spec as written: values and blocks the source omitted stay omitted.
        RepresentationFormat::Spec => dump_spec(&path, format, Walk::Source),
        // The spec with defaults filled in, so an operator can see the values
        // the runtime will apply for settings they did not write.
        RepresentationFormat::PopulatedSpec => dump_spec(&path, format, Walk::Populated),
        RepresentationFormat::Runtime => {
            let cfg = load_config(&path)?.config;
            dump_serde(&cfg, format)
        }
    }
}

fn dump_spec(
    path: &std::path::Path,
    format: ConfigDumpOutputFormat,
    walk: Walk,
) -> anyhow::Result<()> {
    let (_sources, _report, server, devices, ingresses) = load_spec_files(path)?;

    match format {
        ConfigDumpOutputFormat::Hcl => {
            let walked = |spec: &dyn ToFields| match walk {
                Walk::Source => spec.to_source_fields(),
                _ => spec.to_fields(),
            };

            let mut sections = Vec::new();
            sections.push(emit_hcl(&Fields::detached(vec![Field::detached_block(
                "server",
                walked(&server),
            )]))?);
            for device in &devices {
                sections.push(emit_hcl(&walked(&device.value))?);
            }
            for ingress in &ingresses {
                sections.push(emit_hcl(&Fields::detached(vec![Field::detached_block(
                    "ingress",
                    walked(&ingress.value),
                )]))?);
            }
            println!("{}", sections.join("\n"));
            Ok(())
        }
        // The serde formats have no walk, so only the server's defaulted
        // blocks distinguish the two spec representations.
        ConfigDumpOutputFormat::Json | ConfigDumpOutputFormat::Yaml => {
            let server = match walk {
                Walk::Source => server,
                _ => server.populated(),
            };
            dump_serde(&(server, devices, ingresses), format)
        }
    }
}

fn dump_serde<T: Serialize>(value: &T, format: ConfigDumpOutputFormat) -> anyhow::Result<()> {
    let s = match format {
        ConfigDumpOutputFormat::Json => serde_json::to_string_pretty(value)?,
        ConfigDumpOutputFormat::Yaml => serde_yaml::to_string(value)?,
        ConfigDumpOutputFormat::Hcl => to_hcl_string(value)?,
    };
    println!("{s}");
    Ok(())
}

fn to_hcl_string<T: Serialize>(value: &T) -> Result<String, hcl::Error> {
    let mut buf = Vec::new();

    let formatter = Formatter::builder().prefer_ident_keys(true).build(&mut buf);

    let mut serializer = Serializer::with_formatter(formatter);
    serializer.serialize(value)?;

    Ok(String::from_utf8_lossy(&buf).into_owned())
}

#[derive(Clone, Debug, ValueEnum)]
pub(crate) enum RepresentationFormat {
    /// The parsed spec exactly as written, omitting what the source did not set.
    Spec,
    /// The parsed spec with defaults filled in.
    PopulatedSpec,
    Runtime,
}

#[derive(Clone, Debug, ValueEnum)]
pub(crate) enum ConfigDumpOutputFormat {
    Hcl,
    Json,
    Yaml,
}
