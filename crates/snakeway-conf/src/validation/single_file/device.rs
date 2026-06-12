use crate::types::{
    DeviceSpec, validate_identity_device, validate_network_policy_device,
    validate_request_filter_device, validate_request_rate_limiting_device,
    validate_structured_logging_device, validate_wasm_device,
};
use confval::provenance::{Located, Report, Span};

fn report_already_defined(span: Span, report: &mut Report) {
    report.error("device already defined").at(span).emit();
}

fn report_requires_identity(span: Span, report: &mut Report) {
    report
        .error("device requires identity device to be present and enabled")
        .at(span)
        .emit();
}

pub(crate) fn validate_devices(devices: &[Located<DeviceSpec>], report: &mut Report) {
    let mut identity_seen = false;
    let mut identity_enabled = false;
    let mut network_policy_seen = false;
    let mut request_rate_limiting_device_seen = false;
    let mut request_filter_seen = false;
    let mut structured_logging_seen = false;

    // Identity first: other devices depend on it being enabled.
    let enabled_devices = devices.iter().filter(|device| device.value.is_enabled());
    for device in enabled_devices {
        if let DeviceSpec::Identity(cfg) = &device.value {
            if identity_seen {
                report_already_defined(device.span, report);
            }
            identity_seen = true;
            identity_enabled = cfg.enable.value;

            validate_identity_device(cfg, report);

            if cfg.enable_geoip.value
                && cfg.geoip_city_db.is_none()
                && cfg.geoip_isp_db.is_none()
                && cfg.geoip_connection_type_db.is_none()
            {
                report
                    .error("geoip enabled with no dbs specified")
                    .at(cfg.enable_geoip.span)
                    .help("At least one geoip db must be specified")
                    .emit();
            }
        };
    }

    let enabled_devices = devices.iter().filter(|device| device.value.is_enabled());
    for device in enabled_devices {
        match &device.value {
            DeviceSpec::RequestFilter(cfg) => {
                if request_filter_seen {
                    report_already_defined(device.span, report);
                }
                request_filter_seen = true;

                validate_request_filter_device(cfg, report);

                if cfg.max_suspicious_body_bytes.value > cfg.max_body_bytes.value {
                    report
                        .warning(
                            "max_suspicious_body_bytes should not be larger than max_body_bytes",
                        )
                        .at(cfg.max_suspicious_body_bytes.span)
                        .help(
                            "max_suspicious_body_bytes applies to functions that can technically \
                             have a body, but should be treated suspiciously (and thus have a \
                             lower max size than a regular body)",
                        )
                        .emit();
                }
            }
            DeviceSpec::NetworkPolicy(cfg) => {
                if network_policy_seen {
                    report_already_defined(device.span, report);
                }
                network_policy_seen = true;

                if !identity_enabled {
                    report_requires_identity(device.span, report);
                }

                if cfg.cidr_allow.is_empty() {
                    report
                        .error("network policy device requires cidr_allow list to be set")
                        .at(device.span)
                        .emit();
                }

                validate_network_policy_device(cfg, report);
            }
            DeviceSpec::RequestRateLimiting(cfg) => {
                if request_rate_limiting_device_seen {
                    report_already_defined(device.span, report);
                }
                request_rate_limiting_device_seen = true;

                if !identity_enabled {
                    report_requires_identity(device.span, report);
                }

                validate_request_rate_limiting_device(cfg, report);
            }
            DeviceSpec::Wasm(cfg) => {
                validate_wasm_device(cfg, report);
            }
            DeviceSpec::StructuredLogging(cfg) => {
                if structured_logging_seen {
                    report_already_defined(device.span, report);
                }
                structured_logging_seen = true;

                validate_structured_logging_device(cfg, report);

                if cfg.include_identity.value && cfg.identity_fields.is_empty() {
                    report
                        .error("structured logging identity fields cannot be empty")
                        .at(cfg.include_identity.span)
                        .emit();
                }

                if cfg.include_headers.value
                    && cfg.allowed_headers.is_empty()
                    && cfg.redacted_headers.is_empty()
                {
                    report
                        .error("structured logging includes headers but no headers are set")
                        .at(cfg.include_headers.span)
                        .help(
                            "Add headers to allowed_headers or redacted_headers to include \
                             headers in structured logs.",
                        )
                        .emit();
                }
            }
            DeviceSpec::Identity(_) => {
                // Validated in the first pass.
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use crate::types::{
        DeviceSpec, IdentityDeviceSpec, NetworkPolicyDeviceSpec, RequestRateLimitingDeviceSpec,
        StructuredLoggingDeviceSpec, WasmDeviceSpec,
    };
    use crate::validation::validate_devices;
    use confval::provenance::{Located, Report};
    use std::path::PathBuf;

    fn device(spec: DeviceSpec) -> Located<DeviceSpec> {
        Located::detached(spec)
    }

    #[test]
    fn validate_wasm_device_valid() {
        // Arrange
        let mut report = Report::new();
        let dir = tempfile::tempdir().unwrap();
        let wasm_file = dir.path().join("plugin.wasm");
        std::fs::write(&wasm_file, "dummy wasm").unwrap();
        let wasm = device(DeviceSpec::Wasm(WasmDeviceSpec {
            enable: Located::detached(true),
            path: Located::detached(wasm_file),
            ..Default::default()
        }));

        // Act
        validate_devices(&[wasm], &mut report);

        // Assert
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
    }

    #[test]
    fn validate_wasm_device_disabled_skips_validation() {
        // Arrange
        let mut report = Report::new();
        let wasm = device(DeviceSpec::Wasm(WasmDeviceSpec {
            enable: Located::detached(false),
            path: Located::detached(PathBuf::from("/non/existent/path")),
            ..Default::default()
        }));

        // Act
        validate_devices(&[wasm], &mut report);

        // Assert
        assert!(!report.has_issues());
    }

    #[test]
    fn validate_wasm_device_path_empty() {
        // Arrange
        let mut report = Report::new();
        let wasm = device(DeviceSpec::Wasm(WasmDeviceSpec {
            enable: Located::detached(true),
            path: Located::detached(PathBuf::from("")),
            ..Default::default()
        }));

        // Act
        validate_devices(&[wasm], &mut report);

        // Assert
        assert!(report.has_issues());
        let messages: Vec<String> = report.issues().iter().map(|e| e.message.clone()).collect();
        assert!(
            messages
                .iter()
                .any(|m| m.contains("wasm device path is empty"))
        );
        assert!(
            messages
                .iter()
                .any(|m| m.contains("wasm device path does not exist"))
        );
    }

    #[test]
    fn validate_wasm_device_path_does_not_exist() {
        // Arrange
        let mut report = Report::new();
        let wasm = device(DeviceSpec::Wasm(WasmDeviceSpec {
            enable: Located::detached(true),
            path: Located::detached(PathBuf::from("/non/existent/path/to/wasm")),
            ..Default::default()
        }));

        // Act
        validate_devices(&[wasm], &mut report);

        // Assert
        assert!(report.has_issues());
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("wasm device path does not exist"))
        );
    }

    #[test]
    fn duplicate_identity_device_rejected() {
        // Arrange
        let mut report = Report::new();
        let identity = || {
            device(DeviceSpec::Identity(IdentityDeviceSpec {
                enable: Located::detached(true),
                ..Default::default()
            }))
        };

        // Act
        validate_devices(&[identity(), identity()], &mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message == "device already defined")
        );
    }

    #[test]
    fn network_policy_requires_identity() {
        // Arrange
        let mut report = Report::new();
        let network_policy = device(DeviceSpec::NetworkPolicy(NetworkPolicyDeviceSpec {
            enable: Located::detached(true),
            cidr_allow: vec![Located::detached("10.0.0.0/8".to_string())],
            ..Default::default()
        }));

        // Act
        validate_devices(&[network_policy], &mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message == "device requires identity device to be present and enabled")
        );
    }

    #[test]
    fn network_policy_requires_cidr_allow() {
        // Arrange
        let mut report = Report::new();
        let identity = device(DeviceSpec::Identity(IdentityDeviceSpec {
            enable: Located::detached(true),
            ..Default::default()
        }));
        let network_policy = device(DeviceSpec::NetworkPolicy(NetworkPolicyDeviceSpec {
            enable: Located::detached(true),
            cidr_allow: vec![],
            ..Default::default()
        }));

        // Act
        validate_devices(&[identity, network_policy], &mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message == "network policy device requires cidr_allow list to be set")
        );
    }

    #[test]
    fn rate_limiting_requires_identity() {
        // Arrange
        let mut report = Report::new();
        let rate_limiting = device(DeviceSpec::RequestRateLimiting(
            RequestRateLimitingDeviceSpec {
                enable: Located::detached(true),
                max_requests_per_second: Located::detached(100),
                window_seconds: Located::detached(10),
                paths: vec![],
            },
        ));

        // Act
        validate_devices(&[rate_limiting], &mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message == "device requires identity device to be present and enabled")
        );
    }

    #[test]
    fn geoip_without_dbs_rejected() {
        // Arrange
        let mut report = Report::new();
        let identity = device(DeviceSpec::Identity(IdentityDeviceSpec {
            enable: Located::detached(true),
            enable_geoip: Located::detached(true),
            ..Default::default()
        }));

        // Act
        validate_devices(&[identity], &mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message == "geoip enabled with no dbs specified")
        );
    }

    #[test]
    fn structured_logging_identity_fields_empty_rejected() {
        // Arrange
        let mut report = Report::new();
        let logging = device(DeviceSpec::StructuredLogging(StructuredLoggingDeviceSpec {
            enable: Located::detached(true),
            include_identity: Located::detached(true),
            identity_fields: vec![],
            ..Default::default()
        }));

        // Act
        validate_devices(&[logging], &mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message == "structured logging identity fields cannot be empty")
        );
    }

    #[test]
    fn structured_logging_headers_without_lists_rejected() {
        // Arrange
        let mut report = Report::new();
        let logging = device(DeviceSpec::StructuredLogging(StructuredLoggingDeviceSpec {
            enable: Located::detached(true),
            include_headers: Located::detached(true),
            ..Default::default()
        }));

        // Act
        validate_devices(&[logging], &mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message == "structured logging includes headers but no headers are set")
        );
    }
}
