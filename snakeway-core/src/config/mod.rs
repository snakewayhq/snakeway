use anyhow::Context;
use serde::Deserialize;
use std::fs;
use std::str::FromStr;

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    /// e.g. "0.0.0.0:8080"
    pub listen: String,
}

#[derive(Debug, Deserialize)]
pub struct RouteConfig {
    /// URL path prefix, e.g. "/", "/static"
    pub path: String,

    /// Proxy upstream (mutually exclusive with file_dir)
    pub upstream: Option<String>,

    /// Local directory for static files (mutually exclusive with upstream)
    pub file_dir: Option<String>,

    /// Whether to serve index.html for directories
    #[serde(default)]
    pub index: bool,

    /// Static file compression configuration.
    #[serde(default)]
    pub config: StaticFileConfig,
}

impl RouteConfig {
    pub fn validate(&self) -> anyhow::Result<()> {
        match (&self.upstream, &self.file_dir) {
            (Some(_), None) => Ok(()),
            (None, Some(_)) => Ok(()),
            _ => anyhow::bail!(
                "route '{}' must define exactly one of `upstream` or `file_dir`",
                self.path
            ),
        }
    }
}

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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeviceKind {
    Wasm,
    Builtin,
}

#[derive(Debug, Deserialize, Eq, Hash, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BuiltinDeviceKind {
    StructuredLogging,
}

#[derive(Debug, Deserialize)]
pub struct DeviceConfig {
    pub name: String,

    pub kind: DeviceKind,

    /// Required for `kind = "wasm"`
    pub path: Option<String>,

    /// Required for `kind = "builtin"`
    pub builtin: Option<BuiltinDeviceKind>,

    #[serde(default = "default_enabled")]
    pub enabled: bool,

    #[serde(flatten)]
    pub(crate) options: toml::Value,
}

fn default_enabled() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct SnakewayConfig {
    pub server: ServerConfig,
    pub routes: Vec<RouteConfig>,
    #[serde(default)]
    pub devices: Vec<DeviceConfig>,
}

impl SnakewayConfig {
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let contents = fs::read_to_string(path)?;
        let cfg: Self = toml::from_str(&contents)?;
        cfg.validate()?;
        Ok(cfg)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        for route in &self.routes {
            route.validate()?;
        }
        Ok(())
    }
}

impl FromStr for SnakewayConfig {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        let cfg: Self = toml::from_str(s).context("failed to parse Snakeway config from string")?;

        Ok(cfg)
    }
}
