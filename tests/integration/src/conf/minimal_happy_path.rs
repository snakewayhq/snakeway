use crate::conf::ConfigBuilder;
use snakeway_core::testing_api::conf::types::RuntimeConfig;

pub fn minimal_ws_runtime_config() -> RuntimeConfig {
    ConfigBuilder::default().with_ws_ingress().build()
}

pub fn minimal_http_runtime_config() -> RuntimeConfig {
    ConfigBuilder::default().with_http_ingress().build()
}

pub fn minimal_http_runtime_config_with_request_filter() -> RuntimeConfig {
    ConfigBuilder::default()
        .with_http_ingress()
        .with_request_filter_device()
        .build()
}

pub fn minimal_grpc_runtime_config() -> RuntimeConfig {
    ConfigBuilder::default().with_grpc_ingress().build()
}

pub fn minimal_static_file_runtime_config() -> RuntimeConfig {
    ConfigBuilder::default()
        .with_static_file_ingress(false)
        .build()
}

pub fn minimal_https_runtime_config_with_acme() -> RuntimeConfig {
    ConfigBuilder::default().with_https_ingress().build()
}
