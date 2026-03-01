use clap::{Subcommand, ValueEnum};

#[derive(Subcommand, Debug)]
pub enum RouteCmd {
    /// Resolve a URL through the routing table without starting a server
    Solve(RouteSolveArgs),
}

#[derive(clap::Args, Debug)]
pub struct RouteSolveArgs {
    /// Full URL to resolve (must include scheme + host, e.g. http://example.com/api/v1)
    pub url: String,

    /// Path to config directory
    #[arg(long, default_value = "config")]
    pub config: std::path::PathBuf,

    /// HTTP method
    #[arg(long, default_value = "GET")]
    pub method: String,

    /// Request header (repeatable, format: KEY:VALUE)
    #[arg(long = "header", value_name = "KEY:VALUE")]
    pub headers: Vec<String>,

    /// Client IP address for policy evaluation
    #[arg(long)]
    pub client_ip: Option<String>,

    /// Override URL scheme (http or https)
    #[arg(long)]
    pub scheme: Option<String>,

    /// Override URL path
    #[arg(long)]
    pub path: Option<String>,

    /// Override URL query string (no leading '?' required)
    #[arg(long)]
    pub query: Option<String>,

    /// Simulated request body size in bytes
    #[arg(long, default_value = "0")]
    pub body_size: usize,

    /// Deterministic key for hash-based upstream selection
    #[arg(long)]
    pub lb_key: Option<String>,

    /// Force upstream index selection (>= 0)
    #[arg(long)]
    pub lb_index: Option<usize>,

    /// Output format
    #[arg(long, value_enum, default_value = "pretty")]
    pub format: RouteSolveOutputFormat,

    /// Include evaluation trace steps
    #[arg(long, default_value = "false")]
    pub trace: bool,

    /// Verbose output (implies --trace)
    #[arg(long, default_value = "false")]
    pub verbose: bool,
}

#[derive(Clone, Debug, ValueEnum)]
pub enum RouteSolveOutputFormat {
    Pretty,
    Json,
}
