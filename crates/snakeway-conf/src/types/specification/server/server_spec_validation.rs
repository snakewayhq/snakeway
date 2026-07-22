use crate::types::ServerSpec;
use crate::validation::validate_cert_pem;
use confval::diagnostic::Report;
use confval::pipeline::Validate;
use confval::{RangeConstraint, range_constraint};

range_constraint!(THREADS, i64, min: 1, max: 1024);
range_constraint!(DNS_REFRESH_INTERVAL_SECONDS, i64, min: 1, max: 3600, units: "seconds");

/// Entity-level validation for the server section. Runs after parsing (or
/// after programmatic construction), so it must not assume a source file
/// exists; spans come from the `Located` values themselves.
impl Validate for ServerSpec {
    fn validate(&self, report: &mut Report) {
        // Version gate...
        // An unrecognized version means this config targets a different
        // schema, so the v1-specific field checks below would be validating
        // against the wrong rules.
        // Emit only the version error and stop.
        // This is intentional: it does not make sense to validate a config of the wrong version.
        if self.version.value != 1 {
            report
                .error(format!("invalid config version: {}", self.version.value))
                .at(self.version.span)
                .help(
                    "This version of Snakeway is not compatible with this config file. \
                     Please upgrade Snakeway.",
                )
                .emit();
            return;
        }

        if let Some(pid_file) = &self.pid_file
            && let Some(parent) = pid_file.value.parent()
        {
            if !parent.exists() {
                report
                    .error(format!(
                        "pid file parent directory does not exist: {}",
                        pid_file.value.display()
                    ))
                    .at(pid_file.span)
                    .emit();
            } else if !parent.is_dir() {
                report
                    .error(format!(
                        "pid file parent is not a directory: {}",
                        pid_file.value.display()
                    ))
                    .at(pid_file.span)
                    .emit();
            }
        }

        if let Some(ca_file) = &self.ca_file
            && let Err(e) = validate_cert_pem(&ca_file.value)
        {
            report
                .error(format!("server CA file is invalid: {}", e))
                .at(ca_file.span)
                .emit();
        }

        if let Some(threads) = &self.threads {
            THREADS.check_located(threads, "threads", report);
        }

        DNS_REFRESH_INTERVAL_SECONDS.check_located(
            &self.dns_refresh_interval_seconds,
            "dns_refresh_interval_seconds",
            report,
        );

        if let Some(tls_automation) = &self.tls_automation {
            tls_automation.value.validate(report);
        }

        if let Some(observability) = &self.observability {
            observability.value.validate(report);
        }

        if let Some(wasm) = &self.wasm {
            wasm.value.validate(report);
        }

        if let Some(shutdown) = &self.shutdown {
            shutdown.validate(report);
        }

        if let Some(upgrade) = &self.upgrade {
            upgrade.validate(report);
        }

        if let Some(performance) = &self.performance {
            performance.validate(report);
        }

        if let Some(upstream) = &self.upstream {
            upstream.validate(report);
        }
    }
}
