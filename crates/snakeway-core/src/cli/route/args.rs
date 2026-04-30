use clap::{Subcommand, ValueEnum};

#[derive(Subcommand, Debug)]
pub enum RouteCmd {
    /// Resolve a URL through the routing table without starting a server
    Solve(RouteSolveArgs),
}

#[derive(clap::Args, Debug)]
pub(crate) struct RouteSolveArgs {
    /// Full URL to resolve (must include scheme + host, e.g. http://example.com/api/v1)
    pub(crate) url: String,

    /// Path to config directory
    #[arg(long, default_value = "config", env = "SNAKEWAY_CONFIG")]
    pub(crate) config: std::path::PathBuf,

    /// HTTP method
    #[arg(long, default_value = "GET")]
    pub(crate) method: String,

    /// Request header (repeatable, format: KEY:VALUE)
    #[arg(long = "header", value_name = "KEY:VALUE")]
    pub(crate) headers: Vec<String>,

    /// Client IP address for policy evaluation
    #[arg(long)]
    pub(crate) client_ip: Option<String>,

    /// Override URL scheme (http or https)
    #[arg(long)]
    pub(crate) scheme: Option<String>,

    /// Override URL path
    #[arg(long)]
    pub(crate) path: Option<String>,

    /// Override URL query string (no leading '?' required)
    #[arg(long)]
    pub(crate) query: Option<String>,

    /// Simulated request body size in bytes
    #[arg(long, default_value = "0")]
    pub(crate) body_size: usize,

    /// Deterministic key for hash-based upstream selection
    #[arg(long)]
    pub(crate) lb_key: Option<String>,

    /// Force upstream index selection (>= 0)
    #[arg(long)]
    pub(crate) lb_index: Option<usize>,

    /// Output format
    #[arg(long, value_enum, default_value = "pretty")]
    pub(crate) format: RouteSolveOutputFormat,

    /// Include evaluation trace steps
    #[arg(long, default_value = "false")]
    pub(crate) trace: bool,

    /// Verbose output (implies --trace)
    #[arg(long, default_value = "false")]
    pub(crate) verbose: bool,
}

#[derive(Clone, Debug, ValueEnum)]
pub(crate) enum RouteSolveOutputFormat {
    Pretty,
    Json,
}
