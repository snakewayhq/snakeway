use confval::format::ToFields;
use confval::format::hcl::emit_hcl;
use confval::source::Located;
use snakeway_conf::types::{
    BindSpec, DevicesFile, EndpointSpec, IdentityDeviceSpec, IngressSpec, ServiceRouteSpec,
    ServiceSpec, UpstreamSpec,
};
use std::collections::HashMap;
use std::path::PathBuf;

pub(crate) fn generate(
    device_dir_path: PathBuf,
    ingress_dir_path: PathBuf,
    files_to_create: &mut HashMap<PathBuf, String>,
) -> Result<(), anyhow::Error> {
    let identity_device_file = DevicesFile {
        identity_device: Some(Located::detached(IdentityDeviceSpec {
            enable: Located::detached(true),
            enable_user_agent: Located::detached(true),
            ..Default::default()
        })),
        ..Default::default()
    };

    files_to_create.insert(
        device_dir_path.join("identity.hcl"),
        emit_hcl(&identity_device_file.to_template())?,
    );

    let httpbin_ingress_spec = IngressSpec {
        bind: Some(Located::detached(BindSpec {
            interface: Located::detached("loopback".to_string()),
            port: Located::detached(8080),
            ..Default::default()
        })),
        services: vec![Located::detached(ServiceSpec {
            name: Located::detached("httpbin".to_string()),
            routes: vec![Located::detached(ServiceRouteSpec {
                path: Located::detached("/get".to_string()),
                hosts: vec![Located::detached("*".to_string())],
                ..Default::default()
            })],
            upstreams: vec![Located::detached(UpstreamSpec {
                endpoint: Some(Located::detached(EndpointSpec {
                    host: Located::detached("httpbin.org".to_string()),
                    port: Located::detached(80),
                    tls: None,
                })),
                sock: None,
                weight: Located::detached(1),
            })],
            ..Default::default()
        })],
        ..Default::default()
    };

    files_to_create.insert(
        ingress_dir_path.join("httpbin.hcl"),
        emit_hcl(&httpbin_ingress_spec.to_template())?,
    );

    Ok(())
}
