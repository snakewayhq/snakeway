use crate::conf::ConfigBuilder;
use crate::constants::TEST_HOST;
use confval::source::Located;
use snakeway::testing_api::conf::types::{
    CachePolicySpec, CompressionOptsSpec, IngressSpec, StaticFilesSpec, StaticRouteSpec,
};
use std::path::PathBuf;

impl ConfigBuilder {
    pub fn with_static_file_ingress(mut self, directory_listing: bool) -> Self {
        let static_files = Self::make_static_file_spec(directory_listing);
        let bind = Self::make_bind(false);
        let ingress_spec = IngressSpec {
            bind: Some(Located::detached(bind)),
            static_files: vec![Located::detached(static_files)],
            ..Default::default()
        };
        self.ingress_specs.push(ingress_spec);
        self
    }

    pub fn with_static_file_and_service_ingress(mut self) -> Self {
        let static_files = Self::make_static_file_spec(false);
        let service = Self::make_service_spec();
        let bind = Self::make_bind(false);
        let ingress_spec = IngressSpec {
            bind: Some(Located::detached(bind)),
            static_files: vec![Located::detached(static_files)],
            services: vec![Located::detached(service)],
            ..Default::default()
        };
        self.ingress_specs.push(ingress_spec);
        self
    }

    fn make_static_file_spec(directory_listing: bool) -> StaticFilesSpec {
        StaticFilesSpec {
            routes: vec![Located::detached(StaticRouteSpec {
                hosts: vec![Located::detached(TEST_HOST.to_string())],
                path: Located::detached("/".to_string()),
                file_dir: Located::detached(PathBuf::from("/var/www/html")),
                index: Some(Located::detached("index.html".to_string())),
                directory_listing: Located::detached(directory_listing),
                max_file_size: Located::detached(1048576),
                compression: Located::detached(CompressionOptsSpec {
                    small_file_threshold: Located::detached(0),
                    min_gzip_size: Located::detached(1024),
                    min_brotli_size: Located::detached(4096),
                    enable_gzip: Located::detached(true),
                    enable_brotli: Located::detached(true),
                }),
                cache_policy: Located::detached(CachePolicySpec {
                    max_age_seconds: Located::detached(60),
                    public: Located::detached(true),
                    immutable: Located::detached(false),
                }),
            })],
        }
    }
}
