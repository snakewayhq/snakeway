use confval::format::ToFields;
use confval::format::hcl::emit_hcl;
use confval::source::Located;
use snakeway_conf::types::{
    ACME_CHALLENGE_HTTP01, BindSpec, DevicesFile, IdentityDeviceSpec, IngressSpec,
    NetworkPolicyDeviceSpec, RedirectSpec, RequestFilterDeviceSpec, RequestRateLimitingDeviceSpec,
    StructuredLoggingDeviceSpec, TlsTerminationSpec,
};
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
                request_filter_device: Some(Located::detached(RequestFilterDeviceSpec::default())),
                ..Default::default()
            },
        ),
        (
            "identity.hcl",
            DevicesFile {
                identity_device: Some(Located::detached(IdentityDeviceSpec::default())),
                ..Default::default()
            },
        ),
        (
            "network_policy.hcl",
            DevicesFile {
                network_policy_device: Some(Located::detached(NetworkPolicyDeviceSpec::default())),
                ..Default::default()
            },
        ),
        (
            "request_rate_limiting.hcl",
            DevicesFile {
                request_rate_limiting_device: Some(Located::detached(
                    RequestRateLimitingDeviceSpec::default(),
                )),
                ..Default::default()
            },
        ),
        (
            "structured_logging.hcl",
            DevicesFile {
                structured_logging_device: Some(Located::detached(
                    StructuredLoggingDeviceSpec::default(),
                )),
                ..Default::default()
            },
        ),
    ]);

    for (file_name, file_content) in device_files {
        files_to_create.insert(
            device_dir_path.join(file_name),
            emit_hcl(&file_content.to_fields())?,
        );
    }

    let httpbin_ingress_spec = IngressSpec {
        bind: Some(Located::detached(BindSpec {
            interface: Located::detached("loopback".to_string()),
            port: Located::detached(8443),
            tls: Some(Located::detached(TlsTerminationSpec::Acme {
                domains: vec![Located::detached("snakeway.test".to_string())],
                challenge: Located::detached(ACME_CHALLENGE_HTTP01.to_string()),
            })),
            enable_http2: Located::detached(false),
            redirect_http_to_https: Some(Located::detached(RedirectSpec {
                port: Located::detached(5002),
                status: Located::detached(308),
            })),
            ..Default::default()
        })),
        ..Default::default()
    };

    files_to_create.insert(
        ingress_dir_path.join("minimal.hcl"),
        emit_hcl(&httpbin_ingress_spec.to_fields())?,
    );

    Ok(())
}
