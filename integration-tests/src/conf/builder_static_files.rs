use crate::conf::ConfigBuilder;
use snakeway_core::conf::types::{
    CachePolicySpec, CompressionOptsSpec, IngressSpec, ServiceRouteSpec, ServiceSpec,
    StaticFilesSpec, StaticRouteSpec,
};
use std::path::PathBuf;

impl ConfigBuilder {
    pub fn with_static_file_ingress(mut self, directory_listing: bool) -> Self {
        let static_files = Self::make_static_file_spec(directory_listing);
        let bind = Self::make_bind(false);
        let ingress_spec = IngressSpec {
            bind: Some(bind),
            static_files: vec![static_files],
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
            bind: Some(bind),
            static_files: vec![static_files],
            services: vec![service],
            ..Default::default()
        };
        self.ingress_specs.push(ingress_spec);
        self
    }

    fn make_static_file_spec(directory_listing: bool) -> StaticFilesSpec {
        StaticFilesSpec {
            origin: Default::default(),
            routes: vec![StaticRouteSpec {
                origin: Default::default(),
                path: "/".to_string(),
                file_dir: PathBuf::from("/var/www/html"),
                index: Some("index.html".to_string()),
                directory_listing,
                max_file_size: 1048576,
                compression: CompressionOptsSpec {
                    small_file_threshold: 0,
                    min_gzip_size: 1024,
                    min_brotli_size: 4096,
                    enable_gzip: true,
                    enable_brotli: true,
                },
                cache_policy: CachePolicySpec {
                    max_age_seconds: 60,
                    public: true,
                    immutable: false,
                },
            }],
        }
    }
}
