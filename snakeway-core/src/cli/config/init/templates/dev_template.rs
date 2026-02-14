use crate::cli::config::init::device_spec_root::IdentityDeviceSpecRoot;
use crate::conf::types::{BindInterfaceInput, BindSpec, IdentityDeviceSpec, IngressSpec};
use std::collections::HashMap;
use std::path::PathBuf;

pub(crate) fn generate(
    device_dir_path: PathBuf,
    ingress_dir_path: PathBuf,
    files_to_create: &mut HashMap<PathBuf, String>,
) -> Result<(), anyhow::Error> {
    let identity_device_spec: IdentityDeviceSpecRoot = IdentityDeviceSpec::default().into();

    files_to_create.insert(
        device_dir_path.join("identity.hcl"),
        hcl::to_string(&identity_device_spec)?,
    );

    let httpbin_ingress_spec = IngressSpec {
        bind: Some(BindSpec {
            interface: BindInterfaceInput::Keyword("loopback".to_string()),
            port: 8080,
            ..Default::default()
        }),
        ..Default::default()
    };

    files_to_create.insert(
        ingress_dir_path.join("minimal.hcl"),
        hcl::to_string(&httpbin_ingress_spec)?,
    );

    Ok(())
}
