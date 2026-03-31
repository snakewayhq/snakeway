use crate::types::DeviceSpec;
use crate::validation::ValidationReport;
use crate::validation::validate_spec_trait::ValidateSpec;

pub(crate) fn validate_devices(devices: &[DeviceSpec], report: &mut ValidationReport) {
    let mut identity_seen = false;
    let mut identity_enabled = false;
    let mut network_policy_seen = false;
    let mut request_rate_limiting_device_seen = false;
    let mut request_filter_seen = false;
    let mut structured_logging_seen = false;

    // Validate identity device spec first.
    let enabled_devices = devices.iter().filter(|device| device.is_enabled());
    for device in enabled_devices {
        if let DeviceSpec::Identity(cfg) = device {
            if identity_seen {
                report.device_already_defined(device.origin());
            }
            identity_seen = true;
            identity_enabled = cfg.enable;

            cfg.validate(device.origin(), report);

            if cfg.enable_geoip
                && cfg.geoip_city_db.is_none()
                && cfg.geoip_isp_db.is_none()
                && cfg.geoip_connection_type_db.is_none()
            {
                report.geoip_enabled_with_no_dbs_specified(device.origin());
            }
        };
    }

    // Validate remaining device specs (some of which may depend on the presence of the identity).
    let enabled_devices = devices.iter().filter(|device| device.is_enabled());
    for device in enabled_devices {
        match device {
            DeviceSpec::RequestFilter(cfg) => {
                if request_filter_seen {
                    report.device_already_defined(device.origin());
                }
                request_filter_seen = true;

                cfg.validate(device.origin(), report);

                if cfg.max_suspicious_body_bytes > cfg.max_body_bytes {
                    report.warn_max_suspicious_bytes_large_than_max_body_bytes(device.origin());
                }
            }
            DeviceSpec::NetworkPolicy(cfg) => {
                if network_policy_seen {
                    report.device_already_defined(device.origin());
                }
                network_policy_seen = true;

                if !identity_enabled {
                    // The network policy device requires the identity device to be present.
                    // It is a no-op internally if the identity device is not present, but it is
                    // import to validate its presence here to a void network policy silently
                    // being ignored.
                    report.device_requires_identity_device(device.origin());
                }

                if cfg.cidr_allow.is_empty() {
                    report.network_policy_device_requires_cidr_allow(device.origin());
                }

                cfg.validate(device.origin(), report);
            }
            DeviceSpec::RequestRateLimiting(cfg) => {
                if request_rate_limiting_device_seen {
                    report.device_already_defined(device.origin());
                }
                request_rate_limiting_device_seen = true;

                if !identity_enabled {
                    // The request rate limiting device requires the identity device to be present.
                    // It is a no-op internally if the identity device is not present, but it is
                    // import to validate its presence here to a void request rate limiting silently
                    // being ignored.
                    report.device_requires_identity_device(device.origin());
                }

                cfg.validate(device.origin(), report);
            }
            DeviceSpec::Wasm(cfg) => {
                cfg.validate(device.origin(), report);
            }
            DeviceSpec::StructuredLogging(cfg) => {
                if structured_logging_seen {
                    report.device_already_defined(device.origin());
                }
                structured_logging_seen = true;

                if cfg.include_identity && cfg.identity_fields.is_empty() {
                    report.structured_logging_identity_fields_empty(device.origin());
                }

                if cfg.include_headers
                    && cfg.allowed_headers.is_empty()
                    && cfg.redacted_headers.is_empty()
                {
                    report.structured_logging_includes_headers_but_no_headers_set(device.origin());
                }
            }
            DeviceSpec::Identity(_) => {
                // No-op, identity device was already validated.
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
    use crate::validation::{ValidationReport, validate_devices};
    use std::path::PathBuf;

    #[test]
    fn validate_wasm_device_valid() {
        // Arrange
        let mut report = ValidationReport::default();
        let dir = tempfile::tempdir().unwrap();

        let wasm_file = dir.path().join("plugin.wasm");
        std::fs::write(&wasm_file, "dummy wasm").unwrap();

        let device = DeviceSpec::Wasm(WasmDeviceSpec {
            enable: true,
            path: wasm_file,
            ..Default::default()
        });

        // Act
        validate_devices(&[device], &mut report);

        // Assert
        assert!(!report.has_violations());
    }

    #[test]
    fn validate_wasm_device_disabled_skips_validation() {
        // Arrange
        let mut report = ValidationReport::default();
        let device = DeviceSpec::Wasm(WasmDeviceSpec {
            enable: false,
            path: PathBuf::from("/non/existent/path"),
            ..Default::default()
        });
        let devices = vec![device];

        // Act
        validate_devices(&devices, &mut report);

        // Assert
        assert!(!report.has_violations());
    }

    #[test]
    fn validate_wasm_device_path_empty() {
        // Arrange
        let mut report = ValidationReport::default();
        let device = DeviceSpec::Wasm(WasmDeviceSpec {
            enable: true,
            path: PathBuf::from(""),
            ..Default::default()
        });
        let devices = vec![device];

        // Act
        validate_devices(&devices, &mut report);

        // Assert
        assert!(report.has_violations());
        let error_messages: Vec<String> = report.errors.iter().map(|e| e.message.clone()).collect();
        assert!(
            error_messages
                .iter()
                .any(|m| m.contains("wasm device path is empty"))
        );
        assert!(
            error_messages
                .iter()
                .any(|m| m.contains("wasm device path does not exist"))
        );
    }

    #[test]
    fn validate_wasm_device_path_does_not_exist() {
        // Arrange
        let mut report = ValidationReport::default();
        let device = DeviceSpec::Wasm(WasmDeviceSpec {
            enable: true,
            path: PathBuf::from("/non/existent/path/to/wasm"),
            ..Default::default()
        });
        let devices = vec![device];

        // Act
        validate_devices(&devices, &mut report);

        // Assert
        assert!(report.has_violations());
        let error_messages: Vec<String> = report.errors.iter().map(|e| e.message.clone()).collect();
        assert!(
            error_messages
                .iter()
                .any(|m| m.contains("wasm device path does not exist"))
        );
    }

    #[test]
    fn validate_wasm_device_path_is_not_a_file() {
        let mut report = ValidationReport::default();
        let dir = tempfile::tempdir().unwrap();

        let device = DeviceSpec::Wasm(WasmDeviceSpec {
            enable: true,
            path: dir.path().to_path_buf(), // directory, not file
            ..Default::default()
        });

        validate_devices(&[device], &mut report);

        assert!(
            report
                .errors
                .iter()
                .any(|e| e.message.contains("wasm device path is not a file"))
        );
    }

    #[test]
    fn validate_identity_device_valid() {
        let mut report = ValidationReport::default();
        let dir = tempfile::tempdir().unwrap();

        let geoip = dir.path().join("geoip.mmdb");
        std::fs::write(&geoip, "dummy").unwrap();

        let device = DeviceSpec::Identity(IdentityDeviceSpec {
            enable: true,
            trusted_proxies: vec!["127.0.0.1/32".to_string(), "10.0.0.0/8".to_string()],
            enable_geoip: true,
            geoip_city_db: Some(geoip),
            max_x_forwarded_for_length: 1024,
            ..Default::default()
        });

        validate_devices(&[device], &mut report);

        assert!(!report.has_violations());
    }

    #[test]
    fn validate_identity_device_invalid_trusted_proxy() {
        // Arrange
        let mut report = ValidationReport::default();
        let device = DeviceSpec::Identity(IdentityDeviceSpec {
            enable: true,
            trusted_proxies: vec!["not-an-ip".to_string()],
            ..Default::default()
        });
        let devices = vec![device];

        // Act
        validate_devices(&devices, &mut report);

        // Assert
        assert!(report.has_violations());
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.message.contains("invalid trusted proxy: not-an-ip"))
        );
    }

    #[test]
    fn validate_identity_device_trusted_proxy_catch_all_v4() {
        // Arrange
        let mut report = ValidationReport::default();
        let device = DeviceSpec::Identity(IdentityDeviceSpec {
            enable: true,
            trusted_proxies: vec!["0.0.0.0/0".to_string()],
            ..Default::default()
        });
        let devices = vec![device];

        // Act
        validate_devices(&devices, &mut report);

        // Assert
        assert!(report.has_violations());
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.message.contains("must not contain a catch-all network"))
        );
    }

    #[test]
    fn validate_identity_device_trusted_proxy_catch_all_v6() {
        // Arrange
        let mut report = ValidationReport::default();
        let device = DeviceSpec::Identity(IdentityDeviceSpec {
            enable: true,
            trusted_proxies: vec!["::/0".to_string()],
            ..Default::default()
        });
        let devices = vec![device];

        // Act
        validate_devices(&devices, &mut report);

        // Assert
        assert!(report.has_violations());
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.message.contains("must not contain a catch-all network"))
        );
    }

    #[test]
    fn validate_identity_device_trusted_proxy_public_ip_warning() {
        // Arrange
        let mut report = ValidationReport::default();
        let device = DeviceSpec::Identity(IdentityDeviceSpec {
            enable: true,
            trusted_proxies: vec!["8.8.8.8/32".to_string()],
            ..Default::default()
        });
        let devices = vec![device];

        // Act
        validate_devices(&devices, &mut report);

        // Assert
        assert!(report.has_violations());
        assert!(report.warnings.iter().any(|w| {
            w.message
                .contains("should NOT contain a public IP range: 8.8.8.8/32")
        }))
    }

    #[test]
    fn validate_identity_device_geoip_db_empty() {
        // Arrange
        let mut report = ValidationReport::default();
        let device = DeviceSpec::Identity(IdentityDeviceSpec {
            enable: true,
            enable_geoip: true,
            geoip_city_db: Some(PathBuf::from("")),
            ..Default::default()
        });
        let devices = vec![device];

        // Act
        validate_devices(&devices, &mut report);

        // Assert
        assert!(report.has_violations());
        let error_messages: Vec<String> = report.errors.iter().map(|e| e.message.clone()).collect();
        assert!(
            error_messages
                .iter()
                .any(|m| m.contains("geoip db path is empty"))
        );
    }

    #[test]
    fn validate_identity_device_geoip_db_does_not_exist() {
        // Arrange
        let mut report = ValidationReport::default();
        let device = DeviceSpec::Identity(IdentityDeviceSpec {
            enable: true,
            enable_geoip: true,
            geoip_city_db: Some(PathBuf::from("/non/existent/geoip.db")),
            ..Default::default()
        });
        let devices = vec![device];

        // Act
        validate_devices(&devices, &mut report);

        // Assert
        assert!(report.has_violations());
        let error_messages: Vec<String> = report.errors.iter().map(|e| e.message.clone()).collect();
        assert!(
            error_messages
                .iter()
                .any(|m| m.contains("geoip db path does not exist"))
        );
    }

    #[test]
    fn validate_identity_device_geoip_db_is_not_a_file() {
        let mut report = ValidationReport::default();
        let dir = tempfile::tempdir().unwrap();

        let device = DeviceSpec::Identity(IdentityDeviceSpec {
            enable: true,
            enable_geoip: true,
            geoip_city_db: Some(dir.path().to_path_buf()), // directory
            ..Default::default()
        });

        validate_devices(&[device], &mut report);

        assert!(
            report
                .errors
                .iter()
                .any(|e| e.message.contains("geoip db path is not a file"))
        );
    }

    #[test]
    fn duplicate_identity_device_rejected() {
        // Arrange
        let mut report = ValidationReport::default();
        let device_a = DeviceSpec::Identity(IdentityDeviceSpec {
            enable: true,
            ..Default::default()
        });
        let device_b = DeviceSpec::Identity(IdentityDeviceSpec {
            enable: true,
            ..Default::default()
        });

        // Act
        validate_devices(&[device_a, device_b], &mut report);

        // Assert
        assert!(report.has_violations());
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.message.contains("device already defined"))
        );
    }

    #[test]
    fn network_policy_requires_identity_device() {
        // Arrange
        let mut report = ValidationReport::default();
        let device = DeviceSpec::NetworkPolicy(NetworkPolicyDeviceSpec {
            enable: true,
            cidr_allow: vec!["10.0.0.0/8".to_string()],
            ..Default::default()
        });

        // Act
        validate_devices(&[device], &mut report);

        // Assert
        assert!(report.has_violations());
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.message.contains("requires identity device"))
        );
    }

    #[test]
    fn rate_limiting_requires_identity_device() {
        // Arrange
        let mut report = ValidationReport::default();
        let device = DeviceSpec::RequestRateLimiting(RequestRateLimitingDeviceSpec {
            enable: true,
            max_requests_per_second: 100,
            window_seconds: 10,
            ..Default::default()
        });

        // Act
        validate_devices(&[device], &mut report);

        // Assert
        assert!(report.has_violations());
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.message.contains("requires identity device"))
        );
    }

    #[test]
    fn structured_logging_identity_fields_empty() {
        // Arrange
        let mut report = ValidationReport::default();
        let device = DeviceSpec::StructuredLogging(StructuredLoggingDeviceSpec {
            enable: true,
            include_identity: true,
            identity_fields: vec![],
            ..Default::default()
        });

        // Act
        validate_devices(&[device], &mut report);

        // Assert
        assert!(report.has_violations());
        assert!(
            report
                .warnings
                .iter()
                .any(|w| w.message.contains("identity fields cannot be empty"))
        );
    }

    #[test]
    fn structured_logging_headers_without_config() {
        // Arrange
        let mut report = ValidationReport::default();
        let device = DeviceSpec::StructuredLogging(StructuredLoggingDeviceSpec {
            enable: true,
            include_headers: true,
            allowed_headers: vec![],
            redacted_headers: vec![],
            ..Default::default()
        });

        // Act
        validate_devices(&[device], &mut report);

        // Assert
        assert!(report.has_violations());
        assert!(report.warnings.iter().any(|w| {
            w.message
                .contains("includes headers but no headers are set")
        }));
    }

    #[test]
    fn identity_geoip_enabled_without_db_produces_warning() {
        // Arrange
        let mut report = ValidationReport::default();
        let device = DeviceSpec::Identity(IdentityDeviceSpec {
            enable: true,
            enable_geoip: true,
            geoip_city_db: None,
            geoip_isp_db: None,
            geoip_connection_type_db: None,
            ..Default::default()
        });

        // Act
        validate_devices(&[device], &mut report);

        // Assert
        assert!(report.has_violations());
        assert!(
            report
                .warnings
                .iter()
                .any(|w| w.message.contains("geoip enabled with no dbs specified"))
        );
    }
}
