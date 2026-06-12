use crate::cli::config::hcl::{to_hcl_block_string, to_hcl_string};
use confval::provenance::Located;
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
        to_hcl_string(&identity_device_file)?,
    );

    let httpbin_ingress_spec = IngressSpec {
        bind: Some(Located::detached(BindSpec {
            interface: Located::detached("loopback".to_string()),
            port: Located::detached(8080),
            ..Default::default()
        })),
        services: vec![Located::detached(ServiceSpec {
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
        to_hcl_block_string(&httpbin_ingress_spec)?,
    );

    Ok(())
}
