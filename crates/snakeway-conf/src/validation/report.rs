use crate::types::HclOrigin;
use confval::{ValidationIssue, ValidationReport};
use owo_colors::OwoColorize;
use std::net::IpAddr;
use std::path::{Display, Path};

pub(crate) trait ValidationReportExt {
    fn report_error(&mut self, message: String, origin: &HclOrigin, help: Option<String>);
    fn report_warning(&mut self, message: String, origin: &HclOrigin, help: Option<String>);

    // Ingress
    fn missing_bind(&mut self, origin: &HclOrigin) {
        self.report_error(
            "ingress config must have a bind or bind_admin declaration".to_string(),
            origin,
            None,
        );
    }

    // Bind
    fn invalid_bind_addr(&mut self, addr: &str, origin: &HclOrigin) {
        self.report_error(format!("invalid bind address: {}", addr), origin, None);
    }

    fn duplicate_bind_addr(&mut self, addr: &str, origin: &HclOrigin) {
        self.report_error(format!("duplicate bind address: {}", addr), origin, None);
    }

    fn ingress_tls_manual_cert_pair_invalid(&mut self, message: &str, origin: &HclOrigin) {
        self.report_error(
            format!("invalid TLS manual cert pair: {}", message),
            origin,
            Some("Use manual mode instead".to_string()),
        );
    }

    fn acme_tls_requires_domains(&mut self, origin: &HclOrigin) {
        self.report_error("missing domains for ACME TLS".to_string(), origin, None);
    }

    fn admin_bind_does_not_support_acme(&mut self, origin: &HclOrigin) {
        self.report_error(
            "admin bind does not support ACME TLS".to_string(),
            origin,
            None,
        );
    }

    fn http2_requires_tls(&mut self, addr: &str, origin: &HclOrigin) {
        self.report_error(
            format!("HTTP/2 requires TLS: {}", addr),
            origin,
            Some("Enable TLS on the bind or disable HTTP/2.".to_string()),
        );
    }

    fn redirect_http_to_https_requires_tls(&mut self, addr: &str, origin: &HclOrigin) {
        self.report_error(
            format!("redirect_http_to_https requires TLS: {}", addr),
            origin,
            Some("Enable TLS on the bind or remove redirect_http_to_https.".to_string()),
        );
    }

    fn duplicate_redirect_http_to_https_port(&mut self, port: u16, origin: &HclOrigin) {
        self.report_error(
            format!("duplicate redirect_http_to_https port: {}", port),
            origin,
            None,
        );
    }

    fn invalid_port(&mut self, port: u16, origin: &HclOrigin) {
        self.report_error(
            format!("invalid port: {}", port),
            origin,
            Some("ports must be in the range 1–65535".to_string()),
        );
    }

    fn connection_filter_requires_at_least_one_ip_family(&mut self, origin: &HclOrigin) {
        self.report_error(
            "connection_filter must enable at least one IP family".to_string(),
            origin,
            Some("Set ip_family.ipv4 and/or ip_family.ipv6 to true.".to_string()),
        );
    }

    fn invalid_cidr_in_connection_filter_allow_list(&mut self, cidr: &str, origin: &HclOrigin) {
        self.report_error(
            format!("invalid CIDR in connection_filter.cidr.allow: {cidr}"),
            origin,
            Some("CIDR must be a valid IPv4 or IPv6 network (e.g. 10.0.0.0/8).".to_string()),
        );
    }

    fn invalid_cidr_in_connection_filter_deny_list(&mut self, cidr: &str, origin: &HclOrigin) {
        self.report_error(
            format!("invalid CIDR in connection_filter.cidr.deny: {cidr}"),
            origin,
            Some("CIDR must be a valid IPv4 or IPv6 network (e.g. 192.168.0.0/16).".to_string()),
        );
    }

    // Admin Auth
    fn admin_auth_missing(&mut self, origin: &HclOrigin) {
        self.report_error(
            "bind_admin.auth is required".to_string(),
            origin,
            Some(
                "Add an auth block, e.g. auth = { bearer = { token_file = \"/etc/snakeway/admin.tokens\" } }"
                    .to_string(),
            ),
        );
    }

    fn admin_auth_bearer_token_file_io_error(
        &mut self,
        path: &Path,
        message: &str,
        origin: &HclOrigin,
    ) {
        self.report_error(
            format!(
                "bearer token_file could not be read ({}): {}",
                path.display(),
                message
            ),
            origin,
            None,
        );
    }

    fn admin_auth_bearer_token_file_empty(&mut self, path: &Path, origin: &HclOrigin) {
        self.report_error(
            format!("bearer token_file is empty: {}", path.display()),
            origin,
            Some("Add at least one token line (one token per line).".to_string()),
        );
    }

    fn admin_auth_bearer_empty_line(&mut self, path: &Path, line: usize, origin: &HclOrigin) {
        self.report_error(
            format!(
                "bearer token_file {} has an empty line at line {}",
                path.display(),
                line
            ),
            origin,
            Some(
                "Remove the blank line. Lines must be either a token or the end of the file."
                    .to_string(),
            ),
        );
    }

    fn admin_auth_bearer_comment_line(&mut self, path: &Path, line: usize, origin: &HclOrigin) {
        self.report_error(
            format!(
                "bearer token_file {} has a comment at line {}; comments are not permitted",
                path.display(),
                line
            ),
            origin,
            Some("Remove the comment line.".to_string()),
        );
    }

    fn admin_auth_bearer_token_too_short(
        &mut self,
        path: &Path,
        line: usize,
        len: usize,
        min: usize,
        origin: &HclOrigin,
    ) {
        self.report_error(
            format!(
                "bearer token_file {} has a token at line {} that is {} bytes; minimum is {}",
                path.display(),
                line,
                len,
                min
            ),
            origin,
            Some(
                "Generate a token with `openssl rand -hex 32` (or any source of at least 32 bytes of random data)."
                    .to_string(),
            ),
        );
    }

    fn admin_auth_bearer_duplicate_token(
        &mut self,
        path: &Path,
        line: usize,
        first_seen_line: usize,
        origin: &HclOrigin,
    ) {
        self.report_warning(
            format!(
                "bearer token_file {} has a duplicate token at line {} (first seen at line {})",
                path.display(),
                line,
                first_seen_line
            ),
            origin,
            Some("Remove the duplicate entry.".to_string()),
        );
    }

    // Static Files
    fn invalid_static_dir(&mut self, dir: &Path, origin: &HclOrigin) {
        self.report_error(
            format!("invalid static directory: {}", dir.display()),
            origin,
            None,
        );
    }

    fn invalid_static_dir_must_be_absolute(&mut self, dir: &Path, origin: &HclOrigin) {
        self.report_error(
            format!(
                "static file directory must be an absolute path: {}",
                dir.display()
            ),
            origin,
            None,
        );
    }

    // Service
    fn service_has_no_upstreams(&mut self, origin: &HclOrigin) {
        self.report_error("service has no upstream backends".to_string(), origin, None);
    }

    fn invalid_upstream_weight(&mut self, weight: &u32, origin: &HclOrigin) {
        self.report_error(format!("invalid upstream weight: {}", weight), origin, None);
    }

    fn upstream_cannot_have_both_sock_and_endpoint(
        &mut self,
        sock: &str,
        host: &str,
        port: u16,
        origin: &HclOrigin,
    ) {
        self.report_error(
            format!(
                "upstream cannot have both sock {} and endpoint: {}:{}",
                sock, host, port
            ),
            origin,
            None,
        );
    }

    fn upstream_must_have_a_sock_or_endpoint(&mut self, origin: &HclOrigin) {
        self.report_error(
            "invalid upstream - it must have a sock or an endpoint, but neither are defined"
                .to_string(),
            origin,
            Some("Only one can be set.".to_string()),
        );
    }

    fn duplicate_upstream_sock(&mut self, sock: &str, origin: &HclOrigin) {
        self.report_error(format!("duplicate upstream sock: {}", sock), origin, None);
    }

    fn route_has_no_hosts(&mut self, origin: &HclOrigin) {
        self.report_error("route has no hosts".to_string(), origin, None);
    }

    fn upstream_tls_sni_required(&mut self, origin: &HclOrigin) {
        self.report_error("upstream TLS SNI required".to_string(), origin, None);
    }

    fn upstream_tls_sni_must_be_dns(&mut self, origin: &HclOrigin) {
        self.report_error(
            "upstream TLS SNI must be DNS name".to_string(),
            origin,
            None,
        );
    }

    fn upstream_tls_has_invalid_ca_file(&mut self, ca_file: &Path, err: &str, origin: &HclOrigin) {
        self.report_error(
            format!(
                "upstream TLS has invalid CA file ({}): {}",
                ca_file.to_string_lossy(),
                err
            ),
            origin,
            None,
        );
    }

    fn duplicate_route_path(&mut self, path: &str, origin: &HclOrigin) {
        self.report_error(
            format!("duplicate route path within the same listener: {path}"),
            origin,
            Some(
                "Each route path must be unique per listener. Use different path prefixes or move the route to a separate ingress file."
                    .to_string(),
            ),
        );
    }

    fn websocket_route_cannot_be_used_with_http2(&mut self, path: &str, origin: &HclOrigin) {
        self.report_error(
            format!("websocket route cannot be used with HTTP2: {}", path),
            origin,
            None,
        );
    }

    fn invalid_upstream_ip(&mut self, ip: &IpAddr, origin: &HclOrigin) {
        self.report_error(format!("invalid upstream ip: {}", ip), origin, None);
    }

    fn invalid_upstream_hostname(&mut self, hostname: &str, origin: &HclOrigin) {
        self.report_error(
            format!("invalid upstream hostname: {}", hostname),
            origin,
            None,
        );
    }

    // Server
    fn invalid_config_version(&mut self, version: &u32, origin: &HclOrigin) {
        self.report_error(format!("invalid config version: {}", version), origin, None);
    }

    fn pid_file_parent_dir_does_not_exist(&mut self, pid_file: Display, origin: &HclOrigin) {
        self.report_error(
            format!("pid file parent directory does not exist: {}", pid_file),
            origin,
            None,
        );
    }

    fn pid_file_parent_not_a_dir(&mut self, pid_file: Display, origin: &HclOrigin) {
        self.report_error(
            format!("pid file parent is not a directory: {}", pid_file),
            origin,
            None,
        );
    }

    fn server_ca_file_invalid(&mut self, message: &str, origin: &HclOrigin) {
        self.report_error(
            format!("server CA file is invalid: {}", message),
            origin,
            None,
        );
    }

    fn acme_configured_in_ingress_but_server_tls_not_configured(&mut self, origin: &HclOrigin) {
        self.report_error(
            "ACME configured in ingress but server.tls_automation is not configured".to_string(),
            origin,
            None,
        );
    }

    fn server_tls_acme_directory_url_cannot_be_empty(&mut self, origin: &HclOrigin) {
        self.report_error(
            "server TLS ACME directory URL cannot be empty".to_string(),
            origin,
            None,
        );
    }

    fn server_tls_acme_directory_url_must_be_https(&mut self, origin: &HclOrigin) {
        self.report_error(
            "server TLS ACME directory URL must be a valid URL".to_string(),
            origin,
            None,
        );
    }

    fn server_tls_acme_contact_email_cannot_be_empty(&mut self, origin: &HclOrigin) {
        self.report_error(
            "server TLS ACME contact email cannot be empty".to_string(),
            origin,
            Some("It must be a list of 1 or more email addresses".to_string()),
        );
    }

    fn server_tls_acme_ca_file_invalid(
        &mut self,
        ca_file: &Path,
        message: &str,
        origin: &HclOrigin,
    ) {
        self.report_error(
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
        );
    }

    fn server_tls_acme_data_dir_cannot_be_empty(&mut self, origin: &HclOrigin) {
        self.report_error(
            "server TLS ACME data_dir path is required".to_string(),
            origin,
            None,
        );
    }

    fn server_tls_acme_data_dir_is_invalid(&mut self, data_dir: &Path, origin: &HclOrigin) {
        self.report_error(
            format!(
                "server TLS ACME data_dir does not exist or is not a directory: {}",
                data_dir.to_string_lossy()
            ),
            origin,
            None,
        );
    }

    fn server_tls_cert_dir_cannot_be_empty(&mut self, origin: &HclOrigin) {
        self.report_error(
            "server TLS filesystem cert_dir path is required".to_string(),
            origin,
            None,
        );
    }

    fn server_tls_cert_dir_is_invalid(&mut self, cert_dir: &Path, origin: &HclOrigin) {
        self.report_error(
            format!(
                "server TLS cert_dir does not exist or is not a directory: {}",
                cert_dir.to_string_lossy()
            ),
            origin,
            None,
        );
    }

    fn warn_server_tls_configured_with_no_tls_listeners(&mut self, origin: &HclOrigin) {
        self.report_warning(
            "server.tls_automation configured but no TLS listeners defined".to_string(),
            origin,
            None,
        );
    }

    // Observability
    fn otel_endpoint_cannot_be_empty(&mut self, origin: &HclOrigin) {
        self.report_error(
            "observability.otel.endpoint cannot be empty when enabled".to_string(),
            origin,
            Some(
                "Provide the gRPC endpoint for the OTLP exporter (e.g., http://localhost:4317)."
                    .to_string(),
            ),
        );
    }

    fn otel_endpoint_must_be_valid_url(&mut self, origin: &HclOrigin) {
        self.report_error(
            "observability.otel.endpoint must be a valid URL".to_string(),
            origin,
            Some("The endpoint must start with http:// or https://.".to_string()),
        );
    }

    fn otel_service_name_cannot_be_empty(&mut self, origin: &HclOrigin) {
        self.report_error(
            "observability.otel.service_name cannot be empty when enabled".to_string(),
            origin,
            None,
        );
    }

    // Wasm Device
    fn wasm_device_path_is_empty(&mut self, path: Display, origin: &HclOrigin) {
        self.report_error(format!("wasm device path is empty: {}", path), origin, None);
    }

    fn wasm_device_path_does_not_exist(&mut self, path: Display, origin: &HclOrigin) {
        self.report_error(
            format!("wasm device path does not exist: {}", path),
            origin,
            None,
        );
    }

    fn wasm_device_path_is_not_a_file(&mut self, path: Display, origin: &HclOrigin) {
        self.report_error(
            format!("wasm device path is not a file: {}", path),
            origin,
            None,
        );
    }

    // Identity / Device
    fn geoip_enabled_with_no_dbs_specified(&mut self, origin: &HclOrigin) {
        self.report_warning(
            "geoip enabled with no dbs specified".to_string(),
            origin,
            Some("At least one geoip db must be specified".to_string()),
        );
    }

    fn geoip_db_path_is_empty(&mut self, path: Display, origin: &HclOrigin) {
        self.report_error(format!("geoip db path is empty: {}", path), origin, None);
    }

    fn geoip_db_path_does_not_exist(&mut self, path: Display, origin: &HclOrigin) {
        self.report_error(
            format!("geoip db path does not exist: {}", path),
            origin,
            None,
        );
    }

    fn geoip_db_is_not_a_file(&mut self, path: Display, origin: &HclOrigin) {
        self.report_error(
            format!("geoip db path is not a file: {}", path),
            origin,
            None,
        );
    }

    fn ua_parser_regexes_path_is_empty(&mut self, path: Display, origin: &HclOrigin) {
        self.report_error(
            format!("ua_parser_regexes path is empty: {}", path),
            origin,
            None,
        );
    }

    fn ua_parser_regexes_path_does_not_exist(&mut self, path: Display, origin: &HclOrigin) {
        self.report_error(
            format!("ua_parser_regexes path does not exist: {}", path),
            origin,
            Some(
                "Provide a valid path to a ua-parser regexes.yaml file, or remove the setting to use the bundled default."
                    .to_string(),
            ),
        );
    }

    fn ua_parser_regexes_path_is_not_a_file(&mut self, path: Display, origin: &HclOrigin) {
        self.report_error(
            format!("ua_parser_regexes path is not a file: {}", path),
            origin,
            None,
        );
    }

    fn ua_parser_regexes_file_missing_expected_content(
        &mut self,
        path: Display,
        origin: &HclOrigin,
    ) {
        self.report_warning(
            format!(
                "ua_parser_regexes file does not appear to be a valid ua-parser regexes.yaml: {}",
                path
            ),
            origin,
            Some(
                "Expected the file to contain a 'user_agent_parsers' section. See https://github.com/ua-parser/uap-core for the expected format."
                    .to_string(),
            ),
        );
    }

    fn invalid_trusted_proxy(&mut self, proxy: &str, origin: &HclOrigin) {
        self.report_error(format!("invalid trusted proxy: {}", proxy), origin, None);
    }

    fn trusted_proxies_cannot_trust_all_networks(&mut self, origin: &HclOrigin) {
        self.report_error(
            "trusted_proxies must not contain a catch-all network (0.0.0.0/0 or ::/0)".to_string(),
            origin,
            None,
        );
    }

    fn trusted_proxies_contains_a_public_ip_range_warning(
        &mut self,
        network: ipnet::IpNet,
        origin: &HclOrigin,
    ) {
        self.report_warning(
            format!("trusted_proxies should NOT contain a public IP range: {network}"),
            origin,
            None,
        );
    }

    fn device_already_defined(&mut self, origin: &HclOrigin) {
        self.report_error("device already defined".to_string(), origin, None);
    }

    fn device_requires_identity_device(&mut self, origin: &HclOrigin) {
        self.report_error(
            "device requires identity device to be present and enabled".to_string(),
            origin,
            None,
        );
    }

    fn network_policy_device_requires_cidr_allow(&mut self, origin: &HclOrigin) {
        self.report_error(
            "network policy device requires cidr_allow list to be set".to_string(),
            origin,
            None,
        );
    }

    fn invalid_network_policy_cidr(&mut self, cidr: &str, origin: &HclOrigin) {
        self.report_error(
            format!("invalid network policy CIDR: {}", cidr),
            origin,
            None,
        );
    }

    fn device_path_must_start_with_slash(&mut self, path: &str, origin: &HclOrigin) {
        self.report_error(
            format!("device path must start with '/': {path}"),
            origin,
            None,
        );
    }

    fn structured_logging_identity_fields_empty(&mut self, origin: &HclOrigin) {
        self.report_warning(
            "structured logging identity fields cannot be empty".to_string(),
            origin,
            None,
        );
    }

    fn structured_logging_includes_headers_but_no_headers_set(&mut self, origin: &HclOrigin) {
        self.report_warning(
            "structured logging includes headers but no headers are set".to_string(),
            origin,
            Some(
                "Add headers to allowed_headers or redacted_headers to include headers in structured logs."
                    .to_string(),
            ),
        );
    }

    fn invalid_http_method(&mut self, method: &str, origin: &HclOrigin) {
        self.report_error(format!("invalid HTTP method: {}", method), origin, None);
    }

    fn invalid_http_header_name(&mut self, header: &str, origin: &HclOrigin) {
        self.report_error(
            format!("invalid HTTP header name: {}", header),
            origin,
            None,
        );
    }

    fn warn_max_suspicious_bytes_large_than_max_body_bytes(&mut self, origin: &HclOrigin) {
        self.report_warning(
            "max_suspicious_body_bytes should not be larger than max_body_bytes".to_string(),
            origin,
            Some("max_suspicious_body_bytes applies to functions that can technically have a body, but should be treated suspiciously (and thus have a lower max size than a regular body)".to_string()),
        );
    }
}

impl ValidationReportExt for ValidationReport<HclOrigin> {
    fn report_error(&mut self, message: String, origin: &HclOrigin, help: Option<String>) {
        match help {
            Some(h) => self.error(ValidationIssue::error_with_help(message, origin.clone(), h)),
            None => self.error(ValidationIssue::error(message, origin.clone())),
        }
    }

    fn report_warning(&mut self, message: String, origin: &HclOrigin, help: Option<String>) {
        match help {
            Some(h) => self.warning(ValidationIssue::warning_with_help(
                message,
                origin.clone(),
                h,
            )),
            None => self.warning(ValidationIssue::warning(message, origin.clone())),
        }
    }
}

//-----------------------------------------------------------------------------
// Rendering
//-----------------------------------------------------------------------------

pub fn render_json(report: &ValidationReport<HclOrigin>) {
    if !report.has_issues() {
        return;
    }

    #[derive(serde::Serialize)]
    struct IssueJson<'a> {
        severity: &'a str,
        message: &'a str,
        origin: &'a HclOrigin,
        help: &'a Option<String>,
    }

    #[derive(serde::Serialize)]
    struct ReportJson<'a> {
        errors: Vec<IssueJson<'a>>,
        warnings: Vec<IssueJson<'a>>,
    }

    fn to_json<'a>(issue: &'a ValidationIssue<HclOrigin>, severity: &'static str) -> IssueJson<'a> {
        IssueJson {
            severity,
            message: &issue.message,
            origin: &issue.origin,
            help: &issue.help,
        }
    }

    let json = ReportJson {
        errors: report
            .errors()
            .iter()
            .map(|i| to_json(i, "error"))
            .collect(),
        warnings: report
            .warnings()
            .iter()
            .map(|i| to_json(i, "warning"))
            .collect(),
    };

    match serde_json::to_string_pretty(&json) {
        Ok(output) => println!("{}", output),
        Err(e) => eprintln!("failed to serialize validation report: {}", e),
    }
}

pub fn render_plain(report: &ValidationReport<HclOrigin>) {
    if !report.has_issues() {
        return;
    }

    for issue in report.iter() {
        let severity = match issue.severity {
            confval::Severity::Error => "error",
            confval::Severity::Warning => "warning",
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

pub fn render_pretty(report: &ValidationReport<HclOrigin>) {
    if !report.has_issues() {
        return;
    }

    println!(
        "configuration validation failed ({} errors, {} warnings)\n",
        report.errors().len(),
        report.warnings().len()
    );

    let mut by_file = std::collections::BTreeMap::new();

    for issue in report.iter() {
        by_file
            .entry(&issue.origin.file)
            .or_insert(Vec::new())
            .push(issue);
    }

    for (file, issues) in by_file {
        println!("{}", file.display());

        for issue in issues {
            let help = issue
                .help
                .as_ref()
                .map(|h| format!("\n   help: {}", h))
                .unwrap_or_default();

            match issue.severity {
                confval::Severity::Error => {
                    println!("  {}: {}{}", "error".red().bold(), issue.message, help);
                }
                confval::Severity::Warning => {
                    println!("  {}: {}{}", "warning".yellow().bold(), issue.message, help);
                }
            }

            println!();
        }
    }
}
