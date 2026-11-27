//! A config directory writer for process tests.
//!
//! Process level tests (shutdown, reload, upgrade) run the built binary
//! against a real config directory. This helper writes one from a
//! parameterized template: the main bind, the service, optional manual TLS,
//! an optional admin listener, optional filter fragments, and a controlled
//! device.d file order, so a test can name device files in the exact order
//! the include glob reads them.

use crate::constants::{CERT_SERVER_KEY, CERT_SERVER_PEM};
use std::fs;
use std::path::Path;

/// The manual TLS termination the test server certificate provides.
pub struct ManualTls {
    pub cert: String,
    pub key: String,
}

impl Default for ManualTls {
    fn default() -> Self {
        Self {
            cert: CERT_SERVER_PEM.to_string(),
            key: CERT_SERVER_KEY.to_string(),
        }
    }
}

/// A standalone admin listener with bearer token auth.
pub struct AdminListener {
    pub port: u16,
    pub token_file: String,
}

impl Default for AdminListener {
    fn default() -> Self {
        Self {
            port: 9443,
            token_file: "certs/admin.tokens".to_string(),
        }
    }
}

/// The shape of a config directory a process test needs.
pub struct ProcessConfig {
    /// The public listener port.
    pub listener_port: u16,
    /// One upstream port per upstream, in config order.
    pub upstream_ports: Vec<u16>,
    /// The listener's HTTP/2 switch. Pair `true` with `tls_manual`, because
    /// HTTP/2 without TLS fails validation.
    pub enable_http2: bool,
    /// Manual TLS termination on the main listener.
    pub tls_manual: Option<ManualTls>,
    /// A standalone admin listener.
    pub admin: Option<AdminListener>,
    /// HCL fragment for the bind's `connection_filter` block. The caller
    /// supplies its own CIDR content.
    pub connection_filter_hcl: Option<String>,
    /// HCL fragment for the bind's `connection_rate_limiting_filter` block.
    pub rate_limiting_hcl: Option<String>,
    /// HCL fragment appended inside the `server { }` block, for example an
    /// `upgrade { sock = ... }` block for zero-drop upgrade tests.
    pub server_fragment: Option<String>,
    /// The server pid file path, when the test drives signals through it.
    pub pid_file: Option<std::path::PathBuf>,
    /// Device files written into device.d in the given order. Name files
    /// with numeric prefixes (`10-policy.hcl`) to control the glob order.
    pub device_files: Vec<(String, String)>,
    /// Extra ingress.d files for shapes the fields above do not model.
    pub extra_ingress_files: Vec<(String, String)>,
    /// drain_seconds for graceful shutdown tests.
    pub drain_seconds: u64,
    /// force_timeout_seconds for graceful shutdown tests.
    pub force_timeout_seconds: u64,
}

impl ProcessConfig {
    /// Writes the config directory to `dest` and returns the path. Missing
    /// parent directories are created.
    pub fn write_to(&self, dest: &Path) -> std::path::PathBuf {
        let _ = fs::remove_dir_all(dest);
        fs::create_dir_all(dest.join("device.d")).expect("failed to create device.d");
        fs::create_dir_all(dest.join("ingress.d")).expect("failed to create ingress.d");

        fs::write(dest.join("snakeway.hcl"), self.snakeway_hcl())
            .expect("failed to write snakeway.hcl");
        fs::write(dest.join("ingress.d").join("api.hcl"), self.ingress_hcl())
            .expect("failed to write ingress.d/api.hcl");
        if self.admin.is_some() {
            fs::write(dest.join("ingress.d").join("admin.hcl"), self.admin_hcl())
                .expect("failed to write ingress.d/admin.hcl");
        }
        for (name, content) in &self.device_files {
            fs::write(dest.join("device.d").join(name), content)
                .expect("failed to write device file");
        }
        for (name, content) in &self.extra_ingress_files {
            fs::write(dest.join("ingress.d").join(name), content)
                .expect("failed to write extra ingress file");
        }
        dest.to_path_buf()
    }

    fn snakeway_hcl(&self) -> String {
        let pid_file_line = match &self.pid_file {
            Some(path) => format!("  pid_file = \"{}\"\n", path.display()),
            None => String::new(),
        };
        let server_fragment = match &self.server_fragment {
            Some(fragment) => format!("  {fragment}\n"),
            None => String::new(),
        };

        format!(
            "server {{\n  version = 1\n{pid_file_line}{server_fragment}  shutdown {{\n    drain_seconds = {drain}\n    force_timeout_seconds = {force}\n  }}\n}}\n\
             \ninclude {{\n  devices   = \"device.d/*.hcl\"\n  ingresses = \"ingress.d/*.hcl\"\n}}\n",
            pid_file_line = pid_file_line,
            server_fragment = server_fragment,
            drain = self.drain_seconds,
            force = self.force_timeout_seconds,
        )
    }

    fn bind_tls_block(&self) -> String {
        let Some(tls) = &self.tls_manual else {
            return String::new();
        };
        format!(
            "\n  tls {{\n    mode = \"manual\"\n    cert = \"{}\"\n    key = \"{}\"\n  }}\n",
            tls.cert, tls.key
        )
    }

    fn bind_filters(&self) -> String {
        let mut out = String::new();
        if let Some(filter) = &self.connection_filter_hcl {
            out.push_str(&format!("\n  connection_filter {{\n{filter}  }}\n"));
        }
        if let Some(rate) = &self.rate_limiting_hcl {
            out.push_str(&format!(
                "\n  connection_rate_limiting_filter {{\n{rate}  }}\n"
            ));
        }
        out
    }

    fn admin_hcl(&self) -> String {
        let Some(admin) = &self.admin else {
            return String::new();
        };
        format!(
            "bind_admin {{\n  interface = \"127.0.0.1\"\n  port = {port}\n\n  tls {{\n    mode = \"manual\"\n    cert = \"{cert}\"\n    key = \"{key}\"\n  }}\n\n  auth {{\n    bearer {{\n      token_file = \"{token}\"\n    }}\n  }}\n}}\n",
            port = admin.port,
            cert = CERT_SERVER_PEM,
            key = CERT_SERVER_KEY,
            token = admin.token_file,
        )
    }

    fn ingress_hcl(&self) -> String {
        let upstreams = self
            .upstream_ports
            .iter()
            .map(|port| {
                format!(
                    "      {{\n        weight = 1\n        endpoint = {{ host = \"127.0.0.1\", port = {port} }}\n      }},\n"
                )
            })
            .collect::<String>();

        format!(
            "bind {{\n  interface    = \"127.0.0.1\"\n  port         = {port}\n  enable_http2 = {http2}\n{tls}{filters}}}\n\
             \nservices = [\n  {{\n    name = \"api\"\n\
             \n    health_check = {{\n      enable                     = false\n      failure_threshold          = 3\n      unhealthy_cooldown_seconds = 10\n    }}\n\
             \n    circuit_breaker = {{\n      enable                     = false\n      failure_threshold          = 3\n      open_duration_milliseconds = 10000\n      half_open_max_requests     = 1\n      success_threshold          = 2\n      count_http_5xx_as_failure  = false\n    }}\n\
             \n    routes = [\n      {{\n        hosts = [\"snakeway.test\"]\n        path = \"/api\"\n      }}\n    ]\n\
             \n    upstreams = [\n{upstreams}    ]\n  }}\n]\n",
            port = self.listener_port,
            http2 = self.enable_http2,
            tls = self.bind_tls_block(),
            filters = self.bind_filters(),
            upstreams = upstreams,
        )
    }
}
