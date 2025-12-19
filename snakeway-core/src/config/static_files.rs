use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct StaticFileConfig {
    pub max_file_size: u64,
    pub small_file_threshold: u64,
    pub min_gzip_size: u64,
    pub min_brotli_size: u64,
    pub enable_gzip: bool,
    pub enable_brotli: bool,
}

impl Default for StaticFileConfig {
    fn default() -> Self {
        Self {
            max_file_size: 10 * 1024 * 1024,  // 10 MiB
            small_file_threshold: 256 * 1024, // 256 KiB
            min_gzip_size: 1024,              // 1 KiB
            min_brotli_size: 4 * 1024,        // 4 KiB
            enable_gzip: true,
            enable_brotli: true,
        }
    }
}
