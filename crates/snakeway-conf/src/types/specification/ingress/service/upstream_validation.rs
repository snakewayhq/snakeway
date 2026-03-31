use crate::types::{EndpointSpec, EndpointTlsSpec, HostSpec, Origin, UpstreamSpec};
use crate::validation::validator::{is_valid_hostname, is_valid_port};
use crate::validation::{ValidateSpec, ValidationReport};

impl ValidateSpec for UpstreamSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        if self.weight == 0 || self.weight > 1_000 {
            report.invalid_upstream_weight(&self.weight, origin);
        }
    }
}

impl ValidateSpec for EndpointSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        match &self.host {
            HostSpec::Ip(ip) if ip.is_unspecified() || ip.is_multicast() => {
                report.invalid_upstream_ip(ip, origin);
            }
            HostSpec::Hostname(name) if !is_valid_hostname(name) => {
                report.invalid_upstream_hostname(name, origin);
            }
            _ => {}
        }

        if !is_valid_port(self.port) {
            report.invalid_port(self.port, origin);
        }

        if let Some(tls) = &self.tls {
            tls.validate(origin, report);
        }
    }
}

impl ValidateSpec for EndpointTlsSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        if self.sni.trim().is_empty() {
            report.upstream_tls_sni_required(origin);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::types::{EndpointSpec, HostSpec, Origin, UpstreamSpec};
    use crate::validation::{ValidateSpec, ValidationReport};
    use std::net::IpAddr;
    use std::str::FromStr;

    fn minimal_upstream() -> UpstreamSpec {
        UpstreamSpec {
            endpoint: Some(EndpointSpec {
                host: HostSpec::Ip(IpAddr::from_str("127.0.0.1").unwrap()),
                port: 3000,
                tls: None,
            }),
            weight: 1,
            ..Default::default()
        }
    }

    #[test]
    fn weight_greater_than_zero() {
        // Arrange
        let mut report = ValidationReport::default();
        let mut upstream = minimal_upstream();
        upstream.weight = 0;
        let origin = Origin::test("upstream");

        // Act
        upstream.validate(&origin, &mut report);

        // Assert
        let error = report.errors.first().expect("expected at least one error");
        assert!(error.message.contains("invalid upstream weight: 0"));
    }

    #[test]
    fn weight_not_greater_than_1000() {
        // Arrange
        let mut report = ValidationReport::default();
        let mut upstream = minimal_upstream();
        upstream.weight = 1001;
        let origin = Origin::test("upstream");

        // Act
        upstream.validate(&origin, &mut report);

        // Assert
        let error = report.errors.first().expect("expected at least one error");
        assert!(error.message.contains("invalid upstream weight: 1001"));
    }
}
