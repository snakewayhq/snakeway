use crate::types::ServerSpec;
use crate::validation::validate_cert_pem;
use confval::diagnostic::Report;
use confval::pipeline::Validate;
use confval::range_constraint;
use std::net::{Ipv4Addr, Ipv6Addr};

range_constraint!(THREADS, i64, min: 1, max: 1024);
range_constraint!(DNS_REFRESH_INTERVAL_SECONDS, i64, min: 1, max: 3600, units: "seconds");
range_constraint!(SHUTDOWN_DRAIN_SECONDS, i64, min: 0, max: 300, units: "seconds");
range_constraint!(SHUTDOWN_FORCE_TIMEOUT_SECONDS, i64, min: 1, max: 300, units: "seconds");
range_constraint!(UPGRADE_MAX_RETRIES, i64, min: 1, max: 60);
range_constraint!(UPSTREAM_CONNECTION_POOL_SIZE, i64, min: 1, max: 65535);
range_constraint!(PARALLEL_ACCEPTS_PER_LISTENER, i64, min: 1, max: 64);
range_constraint!(UPSTREAM_TIMEOUT_SECONDS, i64, min: 1, max: 3600, units: "seconds");

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
            if let Some(drain) = &shutdown.value.drain_seconds {
                SHUTDOWN_DRAIN_SECONDS.check_located(drain, "drain_seconds", report);
            }
            if let Some(timeout) = &shutdown.value.force_timeout_seconds {
                SHUTDOWN_FORCE_TIMEOUT_SECONDS.check_located(
                    timeout,
                    "force_timeout_seconds",
                    report,
                );
            }
        }

        if let Some(upgrade) = &self.upgrade
            && let Some(retries) = &upgrade.value.max_retries
        {
            UPGRADE_MAX_RETRIES.check_located(retries, "max_retries", report);
        }

        if let Some(performance) = &self.performance
            && let Some(accepts) = &performance.value.parallel_accepts_per_listener
        {
            PARALLEL_ACCEPTS_PER_LISTENER.check_located(
                accepts,
                "parallel_accepts_per_listener",
                report,
            );
        }

        if let Some(upstream) = &self.upstream {
            if let Some(pool_size) = &upstream.value.connection_pool_size {
                UPSTREAM_CONNECTION_POOL_SIZE.check_located(
                    pool_size,
                    "connection_pool_size",
                    report,
                );
            }
            if let Some(timeout) = &upstream.value.connection_timeout_seconds {
                UPSTREAM_TIMEOUT_SECONDS.check_located(
                    timeout,
                    "connection_timeout_seconds",
                    report,
                );
            }
            if let Some(timeout) = &upstream.value.read_timeout_seconds {
                UPSTREAM_TIMEOUT_SECONDS.check_located(timeout, "read_timeout_seconds", report);
            }
        }

        if let Some(upstream) = &self.upstream
            && let Some(source_addrs) = &upstream.value.source_addresses
        {
            for addr in &source_addrs.value.ipv4 {
                if addr.value.parse::<Ipv4Addr>().is_err() {
                    report
                        .error(format!(
                            "invalid upstream.source_addresses.ipv4 entry: \"{}\" is not a valid IPv4 address",
                            addr.value
                        ))
                        .at(addr.span)
                        .emit();
                }
            }
            for addr in &source_addrs.value.ipv6 {
                if addr.value.parse::<Ipv6Addr>().is_err() {
                    report
                        .error(format!(
                            "invalid upstream.source_addresses.ipv6 entry: \"{}\" is not a valid IPv6 address",
                            addr.value
                        ))
                        .at(addr.span)
                        .emit();
                }
            }
        }
    }
}
