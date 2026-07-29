use crate::cli::config::init::templates;
use anyhow::{Context, Result};
use clap::ValueEnum;
use confval::format::ToFields;
use confval::format::hcl::emit_hcl;
use confval::source::Located;
#[cfg(feature = "dev-templates")]
use snakeway_conf::types::{AcmeServerSpec, CertStoreSpec, TlsAutomationSpec};
use snakeway_conf::types::{EntrypointSpec, ServerSpec};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

fn entrypoint_spec(template: &ConfigInitTemplate) -> EntrypointSpec {
    // `spec` is only mutated by the dev-template branch below.
    #[cfg_attr(not(feature = "dev-templates"), allow(unused_mut))]
    // No pid_file by default: a starter config must not assume a host-specific
    // directory exists (validation requires the pid file's parent to be
    // present, which breaks on minimal hosts that lack /var/run).
    // An operator opts into a pid file by setting one explicitly.
    // The dev template sets a portable /tmp path below.
    let mut spec = EntrypointSpec {
        server: ServerSpec {
            threads: Some(Located::detached(8)),
            ..Default::default()
        },
        ..Default::default()
    };
    #[cfg(not(feature = "dev-templates"))]
    let _ = template;
    #[cfg(feature = "dev-templates")]
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
        emit_hcl(&entrypoint_spec(&template).to_template())?,
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
        #[cfg(feature = "dev-templates")]
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
    #[cfg(feature = "dev-templates")]
    Dev,
}

#[cfg(test)]
mod tests {
    use super::*;
    use snakeway_conf::{load_config, load_spec_files};

    /// Every file `config init` writes must parse: the entrypoint plus the
    /// generated `device.d/` and `ingress.d/` files, for every template. This
    /// runs the parse phase over the actual files on disk, not just the
    /// entrypoint string. Parsing is structural only, so it covers the dev
    /// template too (its ACME config parses without any live service).
    #[test]
    fn generated_files_parse_cleanly() {
        for template in [
            ConfigInitTemplate::Minimal,
            ConfigInitTemplate::Httpbin,
            #[cfg(feature = "dev-templates")]
            ConfigInitTemplate::Dev,
        ] {
            // Arrange
            let dir = tempfile::tempdir().expect("tempdir");
            let root = dir.path().join("config");
            init(root.clone(), template.clone()).expect("init writes a config");

            // Act: load_spec_files runs only the parse phase across the tree.
            let (_, report, ..) = load_spec_files(&root)
                .unwrap_or_else(|error| panic!("template {template:?} failed to parse: {error:?}"));

            // Assert
            assert!(
                !report.has_errors(),
                "template {template:?} produced parse errors: {:?}",
                report.issues()
            );
        }
    }

    /// Stronger than parsing alone: the portable templates must survive the
    /// full parse -> validate -> lower pipeline on any host, against the actual
    /// files `init` writes. This is what catches a template baking in a path
    /// that validation rejects (for example a pid file under a directory the
    /// host may not have). The dev template is excluded on purpose: it targets
    /// a local pebble/ACME setup and is gated behind `dev-templates`.
    #[test]
    fn generated_configs_load_cleanly() {
        for template in [ConfigInitTemplate::Minimal, ConfigInitTemplate::Httpbin] {
            // Arrange
            let dir = tempfile::tempdir().expect("tempdir");
            let root = dir.path().join("config");

            // Act
            init(root.clone(), template.clone()).expect("init writes a config");
            let loaded = load_config(&root);

            // Assert
            let validated = loaded.unwrap_or_else(|error| {
                panic!("template {template:?} generated a config that does not load: {error:?}")
            });
            // A starter config bakes in no pid file: the operator opts into one
            // with a path they know exists, rather than inheriting a host
            // assumption from the template.
            assert!(
                validated.config.server.pid_file.as_os_str().is_empty(),
                "template {template:?} should not bake in a pid_file, got {:?}",
                validated.config.server.pid_file
            );
        }
    }
}
