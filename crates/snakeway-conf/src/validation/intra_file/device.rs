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
