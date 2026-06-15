use crate::cli::config::hcl::to_hcl_block_string;
use crate::cli::config::init::templates;
use anyhow::{Context, Result};
use clap::ValueEnum;
use confval::source::Located;
use snakeway_conf::types::{
    AcmeServerSpec, CertStoreSpec, EntrypointSpec, ServerSpec, TlsAutomationSpec,
};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

fn entrypoint_spec(template: &ConfigInitTemplate) -> EntrypointSpec {
    let mut spec = EntrypointSpec {
        server: ServerSpec {
            threads: Some(Located::detached(8)),
            pid_file: Some(Located::detached(PathBuf::from("/var/run/snakeway.pid"))),
            ..Default::default()
        },
        ..Default::default()
    };
    if matches!(template, ConfigInitTemplate::Dev) {
        spec.server.pid_file = Some(Located::detached(PathBuf::from("/tmp/snakeway.pid")));
        spec.server.tls_automation = Some(Located::detached(TlsAutomationSpec {
            acme: Located::detached(AcmeServerSpec {
                directory_url: Located::detached("https://127.0.0.1:14000/dir".to_string()),
                data_dir: Located::detached(PathBuf::from("data/acme/orders")),
                contact_email: vec![Located::detached("admin@snakeway.test".to_string())],
                ca_file: Some(Located::detached(PathBuf::from(
                    "crates/snakeway-tests/certs/pebble-ca.pem",
                ))),
            }),
            cert_store: Located::detached(CertStoreSpec::Filesystem {
                cert_dir: Located::detached(PathBuf::from("data/acme/certs")),
            }),
            renew_within_days: Located::detached(30),
        }));
    }
    spec
}

pub(crate) fn init(path: PathBuf, template: ConfigInitTemplate) -> Result<()> {
    use anyhow::bail;

    // Refuse to overwrite an existing non-empty directory.
    if path.exists() {
        if path.is_file() {
            bail!("{} exists and is not a directory", path.display());
        }

        let is_empty = fs::read_dir(&path)?.next().is_none();
        if !is_empty {
            bail!(
                "config directory '{}' already exists and is not empty",
                path.display()
            );
        }
    }

    let entrypoint_file_path = path.join("snakeway.hcl");
    let device_dir_path = path.join("device.d");
    let ingress_dir_path = path.join("ingress.d");

    let mut created_files = Vec::new();
    let mut files_to_create = HashMap::new();
    files_to_create.insert(
        entrypoint_file_path,
        to_hcl_block_string(&entrypoint_spec(&template))?,
    );

    match template {
        ConfigInitTemplate::Minimal => {
            templates::minimal_template::generate(
                device_dir_path.clone(),
                ingress_dir_path.clone(),
                &mut files_to_create,
            )?;
        }
        ConfigInitTemplate::Httpbin => {
            templates::httpbin_template::generate(
                device_dir_path.clone(),
                ingress_dir_path.clone(),
                &mut files_to_create,
            )?;
        }
        ConfigInitTemplate::Dev => {
            templates::dev_template::generate(
                device_dir_path.clone(),
                ingress_dir_path.clone(),
                &mut files_to_create,
            )?;
        }
    }

    for (dest_path, contents) in files_to_create {
        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create directory {}", parent.display()))?;
        }

        let contents = std::str::from_utf8(contents.as_ref())
            .context("config template is not valid UTF-8")?
            .trim_start();

        fs::write(&dest_path, contents.as_bytes())
            .with_context(|| format!("failed to write {}", dest_path.display()))?;

        created_files.push(dest_path);
    }

    // Sort for deterministic output
    created_files.sort();

    // User feedback
    println!("✔ Initialized Snakeway config in {}", path.display());
    println!("✔ Created:");
    for file in created_files {
        println!("  - {}", file.display());
    }
    println!();
    println!("Next steps:");
    println!("  snakeway config check {}", path.display());
    println!("  snakeway run --config {}", path.display());
    Ok(())
}

#[derive(Debug, ValueEnum, Clone)]
pub(crate) enum ConfigInitTemplate {
    Minimal,
    Httpbin,
    Dev,
}

#[cfg(test)]
mod tests {
    use super::*;
    use confval::format::hcl::parse_hcl;
    use confval::prelude::{Report, SourceMap};

    /// The generated entrypoint and the config parser must never drift
    /// apart: whatever `config init` writes has to parse cleanly.
    #[test]
    fn generated_entrypoints_parse_cleanly() {
        for template in [
            ConfigInitTemplate::Minimal,
            ConfigInitTemplate::Httpbin,
            ConfigInitTemplate::Dev,
        ] {
            // Arrange
            let text = to_hcl_block_string(&entrypoint_spec(&template)).unwrap();

            // Act
            let mut sources = SourceMap::new();
            let mut report = Report::new();
            let id = sources.add("snakeway.hcl", &text);
            let parsed: Option<EntrypointSpec> = parse_hcl(&sources, id, &mut report);

            // Assert
            assert!(
                !report.has_issues(),
                "template {template:?} produced issues: {:?}\ngenerated:\n{text}",
                report.issues()
            );
            let parsed = parsed.unwrap();
            assert_eq!(parsed.server.version.value, 1);
        }
    }
}
