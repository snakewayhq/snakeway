use crate::cli::config::hcl::{to_hcl_block_string, to_hcl_string};
use confval::provenance::Located;
use snakeway_conf::types::{BindSpec, DevicesFile, IdentityDeviceSpec, IngressSpec};
use std::collections::HashMap;
use std::path::PathBuf;

pub(crate) fn generate(
    device_dir_path: PathBuf,
    ingress_dir_path: PathBuf,
    files_to_create: &mut HashMap<PathBuf, String>,
) -> Result<(), anyhow::Error> {
    let identity_device_file = DevicesFile {
        identity_device: Some(Located::detached(IdentityDeviceSpec::default())),
        ..Default::default()
    };
    files_to_create.insert(
        device_dir_path.join("identity.hcl"),
        to_hcl_string(&identity_device_file)?,
    );

    let httpbin_ingress_spec = IngressSpec {
        bind: Some(Located::detached(BindSpec {
            interface: Located::detached("loopback".to_string()),
            port: Located::detached(8080),
            ..Default::default()
        })),
        ..Default::default()
    };

    files_to_create.insert(
        ingress_dir_path.join("minimal.hcl"),
        to_hcl_block_string(&httpbin_ingress_spec)?,
    );

    Ok(())
}
