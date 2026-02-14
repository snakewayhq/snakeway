use crate::conf::types::{
    BindInterfaceInput, BindSpec, DevicesFile, IdentityDeviceSpec, IngressSpec,
    NetworkPolicyDeviceSpec, RequestFilterDeviceSpec, RequestRateLimitingDeviceSpec,
    StructuredLoggingDeviceSpec,
};
use crate::serialization::to_hcl_string;
use std::collections::HashMap;
use std::path::PathBuf;

pub(crate) fn generate(
    device_dir_path: PathBuf,
    ingress_dir_path: PathBuf,
    files_to_create: &mut HashMap<PathBuf, String>,
) -> Result<(), anyhow::Error> {
    let device_files = HashMap::from([
        (
            "request_filter.hcl",
            DevicesFile {
                request_filter_device: Some(RequestFilterDeviceSpec::default()),
                ..Default::default()
            },
        ),
        (
            "identity.hcl",
            DevicesFile {
                identity_device: Some(IdentityDeviceSpec::default()),
                ..Default::default()
            },
        ),
        (
            "network_policy.hcl",
            DevicesFile {
                network_policy_device: Some(NetworkPolicyDeviceSpec::default()),
                ..Default::default()
            },
        ),
        (
            "request_rate_limiting.hcl",
            DevicesFile {
                request_rate_limiting_device: Some(RequestRateLimitingDeviceSpec::default()),
                ..Default::default()
            },
        ),
        (
            "structured_logging.hcl",
            DevicesFile {
                structured_logging_device: Some(StructuredLoggingDeviceSpec::default()),
                ..Default::default()
            },
        ),
    ]);

    for (file_name, file_content) in device_files {
        files_to_create.insert(
            device_dir_path.join(file_name),
            to_hcl_string(&file_content)?,
        );
    }

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
        to_hcl_string(&httpbin_ingress_spec)?,
    );

    Ok(())
}
