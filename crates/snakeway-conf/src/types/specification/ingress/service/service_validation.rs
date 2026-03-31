use crate::types::{Origin, ServiceSpec};
use crate::validation::{ValidateSpec, ValidationReport};
use std::collections::HashSet;

impl ValidateSpec for ServiceSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        // Validate circuit breaker.
        if let Some(cb) = &self.circuit_breaker
            && cb.enable_auto_recovery
        {
            cb.validate(&origin, report);
        }

        // Route (todo refactor to use route's validate() method)
        for route in &self.routes {
            if route.hosts.is_empty() {
                report.route_has_no_hosts(&origin);
            }
        }

        // Upstream
        if self.upstreams.is_empty() {
            report.service_has_no_upstreams(&origin);
        }

        let mut seen_sock_values = HashSet::new();

        for upstream in &self.upstreams {
            upstream.validate(&upstream.origin, report);

            // todo move most of this upstream validation into upstream.validate().
            if let (Some(sock), Some(endpoint)) = (&upstream.sock, &upstream.endpoint) {
                report.upstream_cannot_have_both_sock_and_endpoint(
                    sock,
                    &endpoint.host.to_string(),
                    endpoint.port,
                    &origin,
                );
                continue;
            }

            if upstream.sock.is_none() && upstream.endpoint.is_none() {
                report.upstream_must_have_a_sock_or_endpoint(&origin);
                continue;
            }

            if let Some(endpoint) = &upstream.endpoint {
                endpoint.validate(&upstream.origin, report);

                // Cross-field TLS checks that depend on verify flag.
                if let Some(tls) = &endpoint.tls
                    && tls.verify
                {
                    if tls.sni.parse::<std::net::IpAddr>().is_ok() {
                        report.upstream_tls_sni_must_be_dns(&upstream.origin);
                    }

                    if let Some(ca_file) = &tls.ca_file
                        && let Err(e) = crate::validation::validator::validate_cert_pem(ca_file)
                    {
                        report.upstream_tls_has_invalid_ca_file(ca_file, &e, &upstream.origin);
                    }
                }
            }

            if let Some(sock) = &upstream.sock
                && !seen_sock_values.insert(sock.clone())
            {
                report.duplicate_upstream_sock(sock, &upstream.origin);
            }
        }
    }
}
