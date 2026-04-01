use crate::types::Origin;
use owo_colors::OwoColorize;
use serde::Serialize;
use std::fmt::Debug;
use std::net::IpAddr;
use std::path::{Display, Path};

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

    pub fn error(&mut self, message: String, origin: &Origin, help: Option<String>) {
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

        match serde_json::to_string_pretty(&json) {
            Ok(output) => println!("{}", output),
            Err(e) => eprintln!("failed to serialize validation report: {}", e),
        }
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
    pub(crate) fn missing_bind(&mut self, origin: &Origin) {
        self.error(
            "ingress config must have a bind or bind_admin declaration".to_string(),
            origin,
            None,
        );
    }
}

/// Bind Spec Validation
impl ValidationReport {
    pub(crate) fn invalid_bind_addr(&mut self, addr: &str, origin: &Origin) {
        self.error(format!("invalid bind address: {}", addr), origin, None);
    }

    pub(crate) fn duplicate_bind_addr(&mut self, addr: &str, origin: &Origin) {
        self.error(format!("duplicate bind address: {}", addr), origin, None);
    }

    pub(crate) fn ingress_tls_manual_cert_pair_invalid(&mut self, message: &str, origin: &Origin) {
        self.error(
            format!("invalid TLS manual cert pair: {}", message),
            origin,
            Some("Use manual mode instead".to_string()),
        );
    }

    pub(crate) fn acme_tls_requires_domains(&mut self, origin: &Origin) {
        self.error("missing domains for ACME TLS".to_string(), origin, None);
    }

    pub(crate) fn admin_bind_does_not_support_acme(&mut self, origin: &Origin) {
        self.error(
            "admin bind does not support ACME TLS".to_string(),
            origin,
            None,
        );
    }

    pub(crate) fn http2_requires_tls(&mut self, addr: &str, origin: &Origin) {
        self.error(
            format!("HTTP/2 requires TLS: {}", addr),
            origin,
            Some("Enable TLS on the bind or disable HTTP/2.".to_string()),
        );
    }

    pub(crate) fn redirect_http_to_https_requires_tls(&mut self, addr: &str, origin: &Origin) {
        self.error(
            format!("redirect_http_to_https requires TLS: {}", addr),
            origin,
            Some("Enable TLS on the bind or remove redirect_http_to_https.".to_string()),
        );
    }

    pub(crate) fn duplicate_redirect_http_to_https_port(&mut self, port: u16, origin: &Origin) {
        self.error(
            format!("duplicate redirect_http_to_https port: {}", port),
            origin,
            None,
        );
    }

    pub(crate) fn invalid_port(&mut self, port: u16, origin: &Origin) {
        self.error(
            format!("invalid port: {}", port),
            origin,
            Some("ports must be in the range 1–65535".to_string()),
        );
    }

    pub(crate) fn connection_filter_requires_at_least_one_ip_family(&mut self, origin: &Origin) {
        self.error(
            "connection_filter must enable at least one IP family".to_string(),
            origin,
            Some("Set ip_family.ipv4 and/or ip_family.ipv6 to true.".to_string()),
        );
    }

    pub(crate) fn invalid_cidr_in_connection_filter_allow_list(
        &mut self,
        cidr: &str,
        origin: &Origin,
    ) {
        self.error(
            format!("invalid CIDR in connection_filter.cidr.allow: {cidr}"),
            origin,
            Some("CIDR must be a valid IPv4 or IPv6 network (e.g. 10.0.0.0/8).".to_string()),
        );
    }

    pub(crate) fn invalid_cidr_in_connection_filter_deny_list(
        &mut self,
        cidr: &str,
        origin: &Origin,
    ) {
        self.error(
            format!("invalid CIDR in connection_filter.cidr.deny: {cidr}"),
            origin,
            Some("CIDR must be a valid IPv4 or IPv6 network (e.g. 192.168.0.0/16).".to_string()),
        );
    }
}

/// Static Files Spec Validation
impl ValidationReport {
    pub(crate) fn invalid_static_dir(&mut self, dir: &std::path::Path, origin: &Origin) {
        self.error(
            format!("invalid static directory: {}", dir.display()),
            origin,
            None,
        );
    }

    pub(crate) fn invalid_static_dir_must_be_absolute(
        &mut self,
        dir: &std::path::Path,
        origin: &Origin,
    ) {
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
    pub(crate) fn service_has_no_upstreams(&mut self, origin: &Origin) {
        self.error("service has no upstream backends".to_string(), origin, None)
    }

    pub(crate) fn invalid_upstream_weight(&mut self, weight: &u32, origin: &Origin) {
        self.error(format!("invalid upstream weight: {}", weight), origin, None)
    }

    pub(crate) fn upstream_cannot_have_both_sock_and_endpoint(
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

    pub(crate) fn upstream_must_have_a_sock_or_endpoint(&mut self, origin: &Origin) {
        let message =
            "invalid upstream - it must have a sock or an endpoint, but neither are defined"
                .to_string();
        self.error(message, origin, Some("Only one can be set.".to_string()));
    }

    pub(crate) fn duplicate_upstream_sock(&mut self, sock: &str, origin: &Origin) {
        self.error(format!("duplicate upstream sock: {}", sock), origin, None)
    }

    pub(crate) fn route_has_no_hosts(&mut self, origin: &Origin) {
        self.error("route has no hosts".to_string(), origin, None)
    }

    pub(crate) fn upstream_tls_sni_required(&mut self, origin: &Origin) {
        self.error("upstream TLS SNI required".to_string(), origin, None)
    }

    pub(crate) fn upstream_tls_sni_must_be_dns(&mut self, origin: &Origin) {
        self.error(
            "upstream TLS SNI must be DNS name".to_string(),
            origin,
            None,
        )
    }

    pub(crate) fn upstream_tls_has_invalid_ca_file(
        &mut self,
        ca_file: &Path,
        err: &str,
        origin: &Origin,
    ) {
        self.error(
            format!(
                "upstream TLS has invalid CA file ({}): {}",
                ca_file.to_string_lossy(),
                err
            ),
            origin,
            None,
        )
    }

    pub(crate) fn websocket_route_cannot_be_used_with_http2(
        &mut self,
        path: &str,
        origin: &Origin,
    ) {
        self.error(
            format!("websocket route cannot be used with HTTP2: {}", path),
            origin,
            None,
        )
    }

    pub(crate) fn invalid_upstream_ip(&mut self, ip: &IpAddr, origin: &Origin) {
        self.error(format!("invalid upstream ip: {}", ip), origin, None)
    }

    pub(crate) fn invalid_upstream_hostname(&mut self, hostname: &str, origin: &Origin) {
        self.error(
            format!("invalid upstream hostname: {}", hostname),
            origin,
            None,
        )
    }
}

/// Server Spec Validation
impl ValidationReport {
    pub(crate) fn invalid_config_version(&mut self, version: &u32, origin: &Origin) {
        self.error(format!("invalid config version: {}", version), origin, None)
    }

    pub(crate) fn pid_file_parent_dir_does_not_exist(
        &mut self,
        pid_file: Display,
        origin: &Origin,
    ) {
        self.error(
            format!("pid file parent directory does not exist: {}", pid_file),
            origin,
            None,
        )
    }

    pub(crate) fn pid_file_parent_not_a_dir(&mut self, pid_file: Display, origin: &Origin) {
        self.error(
            format!("pid file parent is not a directory: {}", pid_file),
            origin,
            None,
        )
    }

    pub(crate) fn server_ca_file_invalid(&mut self, message: &str, origin: &Origin) {
        self.error(
            format!("server CA file is invalid: {}", message),
            origin,
            None,
        )
    }

    pub(crate) fn acme_configured_in_ingress_but_server_tls_not_configured(
        &mut self,
        origin: &Origin,
    ) {
        self.error(
            "ACME configured in ingress but server.tls_automation is not configured".to_string(),
            origin,
            None,
        )
    }

    pub(crate) fn server_tls_acme_directory_url_cannot_be_empty(&mut self, origin: &Origin) {
        self.error(
            "server TLS ACME directory URL cannot be empty".to_string(),
            origin,
            None,
        )
    }

    pub(crate) fn server_tls_acme_directory_url_must_be_https(&mut self, origin: &Origin) {
        self.error(
            "server TLS ACME directory URL must be a valid URL".to_string(),
            origin,
            None,
        )
    }

    pub(crate) fn server_tls_acme_contact_email_cannot_be_empty(&mut self, origin: &Origin) {
        self.error(
            "server TLS ACME contact email cannot be empty".to_string(),
            origin,
            Some("It must be a list of 1 or more email addresses".to_string()),
        )
    }

    pub(crate) fn server_tls_acme_ca_file_invalid(
        &mut self,
        ca_file: &Path,
        message: &str,
        origin: &Origin,
    ) {
        self.error(
            format!(
                "server TLS ACME CA file is invalid: {} - {}",
                ca_file.to_string_lossy(),
                message
            ),
            origin,
            Some(
                "In most production scenarios, this should not be set. \
            For example, Let's Encrypt will use a root CA that is already \
            trusted by your operating system. \
            If you are using a custom CA in production or pebble for local development, you should \
            set the server.tls.acme.ca_file option."
                    .to_string(),
            ),
        )
    }

    pub(crate) fn server_tls_acme_data_dir_cannot_be_empty(&mut self, origin: &Origin) {
        self.error(
            "server TLS ACME data_dir path is required".to_string(),
            origin,
            None,
        )
    }

    pub(crate) fn server_tls_acme_data_dir_is_invalid(&mut self, data_dir: &Path, origin: &Origin) {
        self.error(
            format!(
                "server TLS ACME data_dir does not exist or is not a directory: {}",
                data_dir.to_string_lossy()
            ),
            origin,
            None,
        )
    }

    pub(crate) fn server_tls_cert_dir_cannot_be_empty(&mut self, origin: &Origin) {
        self.error(
            "server TLS filesystem cert_dir path is required".to_string(),
            origin,
            None,
        )
    }

    pub(crate) fn server_tls_cert_dir_is_invalid(&mut self, cert_dir: &Path, origin: &Origin) {
        self.error(
            format!(
                "server TLS cert_dir does not exist or is not a directory: {}",
                cert_dir.to_string_lossy()
            ),
            origin,
            None,
        )
    }

    pub(crate) fn warn_server_tls_configured_with_no_tls_listeners(&mut self, origin: &Origin) {
        self.warning(
            "server.tls_automation configured but no TLS listeners defined".to_string(),
            origin,
            None,
        )
    }
}

/// Wasm Device Spec Validation
impl ValidationReport {
    pub(crate) fn wasm_device_path_is_empty(&mut self, path: Display, origin: &Origin) {
        self.error(format!("wasm device path is empty: {}", path), origin, None)
    }
    pub(crate) fn wasm_device_path_does_not_exist(&mut self, path: Display, origin: &Origin) {
        self.error(
            format!("wasm device path does not exist: {}", path),
            origin,
            None,
        )
    }
    pub(crate) fn wasm_device_path_is_not_a_file(&mut self, path: Display, origin: &Origin) {
        self.error(
            format!("wasm device path is not a file: {}", path),
            origin,
            None,
        )
    }
}

/// Builtin Identity Device Spec Validation
impl ValidationReport {
    pub(crate) fn geoip_enabled_with_no_dbs_specified(&mut self, origin: &Origin) {
        self.warning(
            "geoip enabled with no dbs specified".to_string(),
            origin,
            Some("At least one geoip db must be specified".to_string()),
        )
    }

    pub(crate) fn geoip_db_path_is_empty(&mut self, path: Display, origin: &Origin) {
        self.error(format!("geoip db path is empty: {}", path), origin, None)
    }
    pub(crate) fn geoip_db_path_does_not_exist(&mut self, path: Display, origin: &Origin) {
        self.error(
            format!("geoip db path does not exist: {}", path),
            origin,
            None,
        )
    }
    pub(crate) fn geoip_db_is_not_a_file(&mut self, path: Display, origin: &Origin) {
        self.error(
            format!("geoip db path is not a file: {}", path),
            origin,
            None,
        )
    }

    pub(crate) fn ua_parser_regexes_path_is_empty(&mut self, path: Display, origin: &Origin) {
        self.error(
            format!("ua_parser_regexes path is empty: {}", path),
            origin,
            None,
        )
    }

    pub(crate) fn ua_parser_regexes_path_does_not_exist(&mut self, path: Display, origin: &Origin) {
        self.error(
            format!("ua_parser_regexes path does not exist: {}", path),
            origin,
            Some(
                "Provide a valid path to a ua-parser regexes.yaml file, or remove the setting to use the bundled default."
                    .to_string(),
            ),
        )
    }

    pub(crate) fn ua_parser_regexes_path_is_not_a_file(&mut self, path: Display, origin: &Origin) {
        self.error(
            format!("ua_parser_regexes path is not a file: {}", path),
            origin,
            None,
        )
    }

    pub(crate) fn ua_parser_regexes_file_missing_expected_content(
        &mut self,
        path: Display,
        origin: &Origin,
    ) {
        self.warning(
            format!(
                "ua_parser_regexes file does not appear to be a valid ua-parser regexes.yaml: {}",
                path
            ),
            origin,
            Some(
                "Expected the file to contain a 'user_agent_parsers' section. See https://github.com/ua-parser/uap-core for the expected format."
                    .to_string(),
            ),
        )
    }

    pub(crate) fn invalid_trusted_proxy(&mut self, proxy: &str, origin: &Origin) {
        self.error(format!("invalid trusted proxy: {}", proxy), origin, None)
    }

    pub(crate) fn trusted_proxies_cannot_trust_all_networks(&mut self, origin: &Origin) {
        self.error(
            "trusted_proxies must not contain a catch-all network (0.0.0.0/0 or ::/0)".to_string(),
            origin,
            None,
        )
    }

    pub(crate) fn trusted_proxies_contains_a_public_ip_range_warning(
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

    pub(crate) fn device_already_defined(&mut self, origin: &Origin) {
        self.error("device already defined".to_string(), origin, None)
    }

    pub(crate) fn device_requires_identity_device(&mut self, origin: &Origin) {
        self.error(
            "device requires identity device to be present and enabled".to_string(),
            origin,
            None,
        )
    }

    pub(crate) fn network_policy_device_requires_cidr_allow(&mut self, origin: &Origin) {
        self.error(
            "network policy device requires cidr_allow list to be set".to_string(),
            origin,
            None,
        )
    }

    pub(crate) fn invalid_network_policy_cidr(&mut self, cidr: &str, origin: &Origin) {
        self.error(
            format!("invalid network policy CIDR: {}", cidr),
            origin,
            None,
        )
    }

    pub(crate) fn device_path_must_start_with_slash(&mut self, path: &str, origin: &Origin) {
        self.error(
            format!("device path must start with '/': {path}"),
            origin,
            None,
        )
    }

    pub(crate) fn structured_logging_identity_fields_empty(&mut self, origin: &Origin) {
        self.warning(
            "structured logging identity fields cannot be empty".to_string(),
            origin,
            None,
        )
    }

    pub(crate) fn structured_logging_includes_headers_but_no_headers_set(
        &mut self,
        origin: &Origin,
    ) {
        self.warning(
            "structured logging includes headers but no headers are set".to_string(),
            origin,
            Some(
                "Add headers to allowed_headers or redacted_headers to include headers in structured logs."
                    .to_string(),
            ),
        )
    }

    pub(crate) fn invalid_http_method(&mut self, method: &str, origin: &Origin) {
        self.error(format!("invalid HTTP method: {}", method), origin, None)
    }

    pub(crate) fn invalid_http_header_name(&mut self, header: &str, origin: &Origin) {
        self.error(
            format!("invalid HTTP header name: {}", header),
            origin,
            None,
        )
    }

    pub(crate) fn warn_max_suspicious_bytes_large_than_max_body_bytes(&mut self, origin: &Origin) {
        self.warning(
            "max_suspicious_body_bytes should not be larger than max_body_bytes".to_string(),
            origin,
            Some("max_suspicious_body_bytes applies to functions that can technically have a body, but should be treated suspiciously (and thus have a lower max size than a regular body)".to_string()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_violations_false_when_empty() {
        // Arrange
        let report = ValidationReport::default();

        // Act
        let result = report.has_violations();

        // Assert
        assert!(!result);
    }

    #[test]
    fn has_violations_true_with_error() {
        // Arrange
        let mut report = ValidationReport::default();
        report.error("test error".to_string(), &Origin::test("test"), None);

        // Act
        let result = report.has_violations();

        // Assert
        assert!(result);
    }

    #[test]
    fn has_violations_true_with_warning() {
        // Arrange
        let mut report = ValidationReport::default();
        report.warning("test warning".to_string(), &Origin::test("test"), None);

        // Act
        let result = report.has_violations();

        // Assert
        assert!(result);
    }

    #[test]
    fn error_adds_to_errors_vec() {
        // Arrange
        let mut report = ValidationReport::default();

        // Act
        report.error(
            "test message".to_string(),
            &Origin::test("test"),
            Some("help text".to_string()),
        );

        // Assert
        assert_eq!(report.errors.len(), 1);
        assert_eq!(report.errors[0].message, "test message");
        assert_eq!(report.errors[0].origin.section, "test");
        assert_eq!(report.errors[0].help, Some("help text".to_string()));
    }

    #[test]
    fn format_help_with_help_text() {
        // Arrange
        let report = ValidationReport::default();
        let issue = ValidationIssue {
            severity: Severity::Error,
            message: "some error".to_string(),
            origin: Origin::test("test"),
            help: Some("try this instead".to_string()),
        };

        // Act
        let result = report.format_help(&issue);

        // Assert
        assert!(
            result.contains("try this instead"),
            "expected format_help output to contain help text, got: {}",
            result
        );
    }

    #[test]
    fn format_help_without_help_text() {
        // Arrange
        let report = ValidationReport::default();
        let issue = ValidationIssue {
            severity: Severity::Error,
            message: "some error".to_string(),
            origin: Origin::test("test"),
            help: None,
        };

        // Act
        let result = report.format_help(&issue);

        // Assert
        assert!(
            result.is_empty(),
            "expected empty string when no help text, got: {}",
            result
        );
    }

    #[test]
    fn render_json_contains_errors_and_warnings() {
        // Arrange
        let mut report = ValidationReport::default();
        report.error("bad port".to_string(), &Origin::test("bind"), None);
        report.warning("unused field".to_string(), &Origin::test("server"), None);

        let json_struct = ValidationReportJson {
            errors: &report.errors,
            warnings: &report.warnings,
        };

        // Act
        let json = serde_json::to_string_pretty(&json_struct).unwrap();

        // Assert
        assert!(json.contains("bad port"));
        assert!(json.contains("unused field"));
        assert!(json.contains("\"errors\""));
        assert!(json.contains("\"warnings\""));
    }

    #[test]
    fn render_pretty_does_not_panic_with_errors() {
        // Arrange
        let mut report = ValidationReport::default();
        report.error(
            "test error".to_string(),
            &Origin::test("bind"),
            Some("try fixing it".to_string()),
        );

        // Act + Assert (no panic)
        report.render_pretty();
    }

    #[test]
    fn render_pretty_does_not_panic_with_warnings() {
        // Arrange
        let mut report = ValidationReport::default();
        report.warning("test warning".to_string(), &Origin::test("server"), None);

        // Act + Assert (no panic)
        report.render_pretty();
    }
}
