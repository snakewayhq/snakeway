use crate::conf::resolution::ResolveError;
use crate::conf::types::Origin;
use owo_colors::OwoColorize;
use serde::Serialize;
use std::fmt::Debug;
use std::net::IpAddr;
use std::path::Display;

#[derive(Debug, Default, Clone, Serialize)]
pub struct ValidationIssue {
    pub severity: Severity,
    pub message: String,
    pub origin: Origin,
    pub help: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize)]
pub enum Severity {
    #[default]
    Error,
    Warning,
}

#[derive(Debug, Default)]
pub struct ValidationReport {
    pub errors: Vec<ValidationIssue>,
    pub warnings: Vec<ValidationIssue>,
}

#[derive(Serialize)]
struct ValidationReportJson<'a> {
    errors: &'a [ValidationIssue],
    warnings: &'a [ValidationIssue],
}

impl ValidationReport {
    pub fn has_violations(&self) -> bool {
        !self.errors.is_empty() || !self.warnings.is_empty()
    }

    pub(crate) fn error(&mut self, message: String, origin: &Origin, help: Option<String>) {
        self.errors.push(ValidationIssue {
            severity: Severity::Error,
            message,
            origin: origin.clone(),
            help,
        });
    }

    fn warning(&mut self, message: String, origin: &Origin, help: Option<String>) {
        self.warnings.push(ValidationIssue {
            severity: Severity::Warning,
            message,
            origin: origin.clone(),
            help,
        });
    }

    pub fn render_json(&self) {
        if !self.has_violations() {
            return;
        }
        let json = ValidationReportJson {
            errors: &self.errors,
            warnings: &self.warnings,
        };

        println!(
            "{}",
            serde_json::to_string_pretty(&json).expect("failed to serialize validation report")
        );
    }

    pub fn render_plain(&self) {
        if !self.has_violations() {
            return;
        }

        for issue in self.errors.iter().chain(self.warnings.iter()) {
            let severity = match issue.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
            };

            println!(
                "{}:{}: {}",
                issue.origin.file.display(),
                severity,
                issue.message
            );

            if let Some(help) = &issue.help {
                println!("  help: {}", help);
            }
        }
    }

    fn format_help(&self, issue: &ValidationIssue) -> String {
        let help = issue.help.as_deref().unwrap_or("");
        let help = if !help.is_empty() {
            &format!("\n   help: {}", help)
        } else {
            ""
        };
        help.to_string()
    }

    pub fn render_pretty(&self) {
        if !self.has_violations() {
            return;
        }

        // Establish that there are some errors and/or warnings.
        println!(
            "configuration validation failed ({} errors, {} warnings)\n",
            self.errors.len(),
            self.warnings.len()
        );

        // Group violations by config file.
        let mut by_file = std::collections::BTreeMap::new();

        // Errors...
        for issue in &self.errors {
            by_file
                .entry(&issue.origin.file)
                .or_insert(Vec::new())
                .push(issue);
        }

        // Warnings...
        for issue in &self.warnings {
            by_file
                .entry(&issue.origin.file)
                .or_insert(Vec::new())
                .push(issue);
        }

        // Render each file's violations in order.
        for (file, issues) in by_file {
            println!("{}", file.display());

            for issue in issues {
                match issue.severity {
                    Severity::Error => {
                        println!(
                            "  {}: {}{}",
                            "error".red().bold(),
                            issue.message,
                            self.format_help(issue)
                        );
                    }
                    Severity::Warning => {
                        println!(
                            "  {}: {}{}",
                            "warning".yellow().bold(),
                            issue.message,
                            self.format_help(issue)
                        );
                    }
                }

                println!();
            }
        }
    }
}

/// Ingress Spec Validation
impl ValidationReport {
    pub fn missing_bind(&mut self, origin: &Origin) {
        self.error(
            "ingress config must have a bind or bind_admin declaration".to_string(),
            origin,
            None,
        );
    }
}

/// Bind Spec Validation
impl ValidationReport {
    pub fn invalid_bind_addr(&mut self, addr: &str, origin: &Origin) {
        self.error(format!("invalid bind address: {}", addr), origin, None);
    }

    pub fn duplicate_bind_addr(&mut self, addr: &str, origin: &Origin) {
        self.error(format!("duplicate bind address: {}", addr), origin, None);
    }

    pub fn missing_cert_file(&mut self, cert_file: &str, origin: &Origin) {
        self.error(format!("missing cert file: {}", cert_file), origin, None);
    }

    pub fn missing_key_file(&mut self, key_file: &str, origin: &Origin) {
        self.error(format!("missing key file: {}", key_file), origin, None);
    }

    pub fn http2_requires_tls(&mut self, addr: &str, origin: &Origin) {
        self.error(
            format!("HTTP/2 requires TLS: {}", addr),
            origin,
            Some("Enable TLS on the bind or disable HTTP/2.".to_string()),
        );
    }

    pub fn redirect_http_to_https_requires_tls(&mut self, addr: &str, origin: &Origin) {
        self.error(
            format!("redirect_http_to_https requires TLS: {}", addr),
            origin,
            Some("Enable TLS on the bind or remove redirect_http_to_https.".to_string()),
        );
    }

    pub fn redirect_status_is_not_a_3xx_code(&mut self, status_code: u16, origin: &Origin) {
        self.error(
            format!("redirect status {status_code} is not a 3xx code"),
            origin,
            None,
        );
    }

    pub fn invalid_http_status_code(&mut self, status_code: u16, origin: &Origin) {
        self.error(
            format!("invalid HTTP status code {}", status_code),
            origin,
            None,
        );
    }

    pub fn duplicate_redirect_http_to_https_port(&mut self, port: u16, origin: &Origin) {
        self.error(
            format!("duplicate redirect_http_to_https port: {}", port),
            origin,
            None,
        );
    }

    pub fn invalid_port(&mut self, port: u16, origin: &Origin) {
        self.error(
            format!("invalid port: {}", port),
            origin,
            Some("ports must be in the range 1–65535".to_string()),
        );
    }
}

/// Static Files Spec Validation
impl ValidationReport {
    pub fn invalid_static_dir(&mut self, dir: &std::path::Path, origin: &Origin) {
        self.error(
            format!("invalid static directory: {}", dir.display()),
            origin,
            None,
        );
    }

    pub fn invalid_static_dir_must_be_absolute(&mut self, dir: &std::path::Path, origin: &Origin) {
        self.error(
            format!(
                "static file directory must be an absolute path: {}",
                dir.display()
            ),
            origin,
            None,
        );
    }
}

/// Service Spec Validation
impl ValidationReport {
    pub fn service_has_no_upstreams(&mut self, origin: &Origin) {
        self.error("service has no upstream backends".to_string(), origin, None)
    }

    pub fn invalid_upstream_weight(&mut self, weight: &u32, origin: &Origin) {
        self.error(format!("invalid upstream weight: {}", weight), origin, None)
    }

    pub fn upstream_cannot_have_both_sock_and_endpoint(
        &mut self,
        sock: &str,
        host: &str,
        port: u16,
        origin: &Origin,
    ) {
        self.error(
            format!(
                "upstream cannot have both sock {} and endpoint: {}:{}",
                sock, host, port
            ),
            origin,
            None,
        )
    }

    pub fn upstream_must_have_a_sock_or_endpoint(&mut self, origin: &Origin) {
        let message =
            "invalid upstream - it must have a sock or an endpoint, but neither are defined"
                .to_string();
        self.error(message, origin, Some("Only one can be set.".to_string()));
    }

    pub fn invalid_upstream_addr(&mut self, err: &ResolveError, origin: &Origin) {
        self.error(format!("invalid upstream address: {:?}", err), origin, None)
    }

    pub fn duplicate_upstream_sock(&mut self, sock: &str, origin: &Origin) {
        self.error(format!("duplicate upstream sock: {}", sock), origin, None)
    }

    pub fn websocket_route_cannot_be_used_with_http2(&mut self, path: &str, origin: &Origin) {
        self.error(
            format!("websocket route cannot be used with HTTP2: {}", path),
            origin,
            None,
        )
    }

    pub fn invalid_upstream_ip(&mut self, ip: &IpAddr, origin: &Origin) {
        self.error(format!("invalid upstream ip: {}", ip), origin, None)
    }

    pub fn invalid_upstream_hostname(&mut self, hostname: &str, origin: &Origin) {
        self.error(
            format!("invalid upstream hostname: {}", hostname),
            origin,
            None,
        )
    }
}

/// Server Spec Validation
impl ValidationReport {
    pub fn invalid_config_version(&mut self, version: &u32, origin: &Origin) {
        self.error(format!("invalid config version: {}", version), origin, None)
    }

    pub fn pid_file_parent_dir_does_not_exist(&mut self, pid_file: Display, origin: &Origin) {
        self.error(
            format!("pid file parent directory does not exist: {}", pid_file),
            origin,
            None,
        )
    }

    pub fn pid_file_parent_not_a_dir(&mut self, pid_file: Display, origin: &Origin) {
        self.error(
            format!("pid file parent is not a directory: {}", pid_file),
            origin,
            None,
        )
    }

    pub fn root_ca_file_does_not_exist(&mut self, ca_file: &str, origin: &Origin) {
        self.error(
            format!("root CA file does not exist: {}", ca_file),
            origin,
            None,
        )
    }

    pub fn root_ca_file_not_a_file(&mut self, ca_file: &str, origin: &Origin) {
        self.error(
            format!("root CA file is not a file: {}", ca_file),
            origin,
            None,
        )
    }
}

/// Wasm Device Spec Validation
impl ValidationReport {
    pub fn wasm_device_path_is_empty(&mut self, path: Display, origin: &Origin) {
        self.error(format!("wasm device path is empty: {}", path), origin, None)
    }
    pub fn wasm_device_path_does_not_exist(&mut self, path: Display, origin: &Origin) {
        self.error(
            format!("wasm device path does not exist: {}", path),
            origin,
            None,
        )
    }
    pub fn wasm_device_path_is_not_a_file(&mut self, path: Display, origin: &Origin) {
        self.error(
            format!("wasm device path is not a file: {}", path),
            origin,
            None,
        )
    }
}

/// Builtin Identity Device Spec Validation
impl ValidationReport {
    pub fn geoip_db_path_is_empty(&mut self, path: Display, origin: &Origin) {
        self.error(format!("geoip db path is empty: {}", path), origin, None)
    }
    pub fn geoip_db_path_does_not_exist(&mut self, path: Display, origin: &Origin) {
        self.error(
            format!("geoip db path does not exist: {}", path),
            origin,
            None,
        )
    }
    pub fn geoip_db_is_not_a_file(&mut self, path: Display, origin: &Origin) {
        self.error(
            format!("geoip db path is not a file: {}", path),
            origin,
            None,
        )
    }

    pub fn invalid_trusted_proxy(&mut self, proxy: &str, origin: &Origin) {
        self.error(format!("invalid trusted proxy: {}", proxy), origin, None)
    }

    pub fn trusted_proxies_cannot_trust_all_networks(&mut self, origin: &Origin) {
        self.error(
            "trusted_proxies must not contain a catch-all network (0.0.0.0/0 or ::/0)".to_string(),
            origin,
            None,
        )
    }

    pub fn trusted_proxies_contains_a_public_ip_range_warning(
        &mut self,
        network: ipnet::IpNet,
        origin: &Origin,
    ) {
        self.warning(
            format!("trusted_proxies should NOT contain a public IP range: {network}"),
            origin,
            None,
        )
    }
}
