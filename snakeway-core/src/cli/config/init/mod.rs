use crate::conf::types::{
    BindInterfaceInput, BindSpec, EndpointSpec, EntrypointSpec, HostSpec, IdentityDeviceSpec,
    IngressSpec, ServerSpec, ServiceRouteSpec, ServiceSpec, UpstreamSpec,
};
use ahash::{HashMap, HashMapExt};
use anyhow::{Context, Result};
use clap::ValueEnum;
use std::fs;
use std::path::PathBuf;

pub fn init(path: PathBuf, template: ConfigInitTemplate) -> Result<()> {
    use anyhow::bail;

    // Refuse to overwrite an existing non-empty directory
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
    let entrypoint_spec = EntrypointSpec {
        server: ServerSpec {
            threads: Some(8),
            pid_file: Some(PathBuf::from("/var/run/snakeway.pid")),
            ..Default::default()
        },
        ..Default::default()
    };
    files_to_create.insert(entrypoint_file_path, hcl::to_string(&entrypoint_spec)?);

    match template {
        ConfigInitTemplate::Default => {}
        ConfigInitTemplate::Httpbin => {
            let identity_device_spec = IdentityDeviceSpec {
                enable: true,
                trusted_proxies: vec![],
                max_x_forwarded_for_length: 1024,
                enable_geoip: false,
                geoip_city_db: None,
                geoip_isp_db: None,
                geoip_connection_type_db: None,
                enable_user_agent: true,
                max_user_agent_length: 2048,
                ..Default::default()
            };

            files_to_create.insert(
                device_dir_path.join("identity_device.hcl"),
                hcl::to_string(&identity_device_spec)?,
            );

            let httpbin_ingress_spec = IngressSpec {
                bind: Some(BindSpec {
                    interface: BindInterfaceInput::Keyword("loopback".to_string()),
                    port: 8080,
                    ..Default::default()
                }),
                services: vec![{
                    ServiceSpec {
                        routes: vec![ServiceRouteSpec {
                            path: "/get".to_string(),
                            ..Default::default()
                        }],
                        upstreams: vec![UpstreamSpec {
                            endpoint: Some(EndpointSpec {
                                host: HostSpec::Hostname("httpbin.org".to_string()),
                                port: 80,
                            }),
                            weight: 1,
                            ..Default::default()
                        }],
                        ..Default::default()
                    }
                }],
                ..Default::default()
            };
            files_to_create.insert(
                ingress_dir_path.join("httpbin_ingress.hcl"),
                hcl::to_string(&httpbin_ingress_spec)?,
            );
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
pub enum ConfigInitTemplate {
    Default,
    Httpbin,
}
