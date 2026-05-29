use crate::types::HclOrigin;
use confval::ValidationIssue;
use std::path::{Display, Path};

pub(crate) fn invalid_config_version(
    version: &i64,
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error(
        format!("invalid config version: {}", version),
        origin.clone(),
    )
}

pub(crate) fn pid_file_parent_dir_does_not_exist(
    pid_file: Display,
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error(
        format!("pid file parent directory does not exist: {}", pid_file),
        origin.clone(),
    )
}

pub(crate) fn pid_file_parent_not_a_dir(
    pid_file: Display,
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error(
        format!("pid file parent is not a directory: {}", pid_file),
        origin.clone(),
    )
}

pub(crate) fn server_ca_file_invalid(
    message: &str,
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error(
        format!("server CA file is invalid: {}", message),
        origin.clone(),
    )
}

pub(crate) fn acme_configured_in_ingress_but_server_tls_not_configured(
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error(
        "ACME configured in ingress but server.tls_automation is not configured",
        origin.clone(),
    )
}

pub(crate) fn server_tls_acme_directory_url_cannot_be_empty(
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error(
        "server TLS ACME directory URL cannot be empty",
        origin.clone(),
    )
}

pub(crate) fn server_tls_acme_directory_url_must_be_https(
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error(
        "server TLS ACME directory URL must be a valid URL",
        origin.clone(),
    )
}

pub(crate) fn server_tls_acme_contact_email_cannot_be_empty(
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error_with_help(
        "server TLS ACME contact email cannot be empty",
        origin.clone(),
        "It must be a list of 1 or more email addresses",
    )
}

pub(crate) fn server_tls_acme_ca_file_invalid(
    ca_file: &Path,
    message: &str,
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error_with_help(
        format!(
            "server TLS ACME CA file is invalid: {} - {}",
            ca_file.to_string_lossy(),
            message
        ),
        origin.clone(),
        "In most production scenarios, this should not be set. \
        For example, Let's Encrypt will use a root CA that is already \
        trusted by your operating system. \
        If you are using a custom CA in production or pebble for local development, you should \
        set the server.tls.acme.ca_file option.",
    )
}

pub(crate) fn server_tls_acme_data_dir_cannot_be_empty(
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error("server TLS ACME data_dir path is required", origin.clone())
}

pub(crate) fn server_tls_acme_data_dir_is_invalid(
    data_dir: &Path,
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error(
        format!(
            "server TLS ACME data_dir does not exist or is not a directory: {}",
            data_dir.to_string_lossy()
        ),
        origin.clone(),
    )
}

pub(crate) fn server_tls_cert_dir_cannot_be_empty(
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error(
        "server TLS filesystem cert_dir path is required",
        origin.clone(),
    )
}

pub(crate) fn server_tls_cert_dir_is_invalid(
    cert_dir: &Path,
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error(
        format!(
            "server TLS cert_dir does not exist or is not a directory: {}",
            cert_dir.to_string_lossy()
        ),
        origin.clone(),
    )
}

pub(crate) fn warn_server_tls_configured_with_no_tls_listeners(
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::warning(
        "server.tls_automation configured but no TLS listeners defined",
        origin.clone(),
    )
}

pub(crate) fn otel_endpoint_cannot_be_empty(origin: &HclOrigin) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error_with_help(
        "observability.otel.endpoint cannot be empty when enabled",
        origin.clone(),
        "Provide the gRPC endpoint for the OTLP exporter (e.g., http://localhost:4317).",
    )
}

pub(crate) fn otel_endpoint_must_be_valid_url(origin: &HclOrigin) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error_with_help(
        "observability.otel.endpoint must be a valid URL",
        origin.clone(),
        "The endpoint must start with http:// or https://.",
    )
}

pub(crate) fn otel_service_name_cannot_be_empty(origin: &HclOrigin) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error(
        "observability.otel.service_name cannot be empty when enabled",
        origin.clone(),
    )
}
