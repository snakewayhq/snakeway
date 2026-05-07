use super::service_issues;
use crate::types::specification::ingress::bind::bind_issues;
use crate::types::{EndpointSpec, EndpointTlsSpec, HclOrigin, HostSpec, UpstreamSpec};
use crate::validation::validator::{is_valid_hostname, is_valid_port};
use confval::{ValidateSpec, ValidationReport};

impl ValidateSpec<HclOrigin> for UpstreamSpec {
    fn validate(&self, origin: &HclOrigin, report: &mut ValidationReport<HclOrigin>) {
        if self.weight == 0 || self.weight > 1_000 {
            report.push(service_issues::invalid_upstream_weight(
                &self.weight,
                origin,
            ));
        }
    }
}

impl ValidateSpec<HclOrigin> for EndpointSpec {
    fn validate(&self, origin: &HclOrigin, report: &mut ValidationReport<HclOrigin>) {
        match &self.host {
            HostSpec::Ip(ip) if ip.is_unspecified() || ip.is_multicast() => {
                report.push(service_issues::invalid_upstream_ip(ip, origin));
            }
            HostSpec::Hostname(name) if !is_valid_hostname(name) => {
                report.push(service_issues::invalid_upstream_hostname(name, origin));
            }
            _ => {}
        }

        if !is_valid_port(self.port) {
            report.push(bind_issues::invalid_port(self.port, origin));
        }

        if let Some(tls) = &self.tls {
            tls.validate(origin, report);
        }
    }
}

impl ValidateSpec<HclOrigin> for EndpointTlsSpec {
    fn validate(&self, origin: &HclOrigin, report: &mut ValidationReport<HclOrigin>) {
        if self.sni.trim().is_empty() {
            report.push(service_issues::upstream_tls_sni_required(origin));
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::types::{EndpointSpec, EndpointTlsSpec, HclOrigin, HostSpec, UpstreamSpec};
    use confval::{ValidateSpec, ValidationReport};
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
        let origin = HclOrigin::test("upstream");

        // Act
        upstream.validate(&origin, &mut report);

        // Assert
        let error = report
            .errors()
            .first()
            .expect("expected at least one error");
        assert!(error.message.contains("invalid upstream weight: 0"));
    }

    #[test]
    fn weight_not_greater_than_1000() {
        // Arrange
        let mut report = ValidationReport::default();
        let mut upstream = minimal_upstream();
        upstream.weight = 1001;
        let origin = HclOrigin::test("upstream");

        // Act
        upstream.validate(&origin, &mut report);

        // Assert
        let error = report
            .errors()
            .first()
            .expect("expected at least one error");
        assert!(error.message.contains("invalid upstream weight: 1001"));
    }

    #[test]
    fn endpoint_ip_unspecified_rejected() {
        // Arrange
        let mut report = ValidationReport::default();
        let endpoint = EndpointSpec {
            host: HostSpec::Ip(IpAddr::from_str("0.0.0.0").unwrap()),
            port: 3000,
            tls: None,
        };
        let origin = HclOrigin::test("endpoint");

        // Act
        endpoint.validate(&origin, &mut report);

        // Assert
        let error = report
            .errors()
            .first()
            .expect("expected at least one error");
        assert!(error.message.contains("invalid upstream ip: 0.0.0.0"));
    }

    #[test]
    fn endpoint_ip_multicast_rejected() {
        // Arrange
        let mut report = ValidationReport::default();
        let endpoint = EndpointSpec {
            host: HostSpec::Ip(IpAddr::from_str("224.0.0.1").unwrap()),
            port: 3000,
            tls: None,
        };
        let origin = HclOrigin::test("endpoint");

        // Act
        endpoint.validate(&origin, &mut report);

        // Assert
        let error = report
            .errors()
            .first()
            .expect("expected at least one error");
        assert!(error.message.contains("invalid upstream ip: 224.0.0.1"));
    }

    #[test]
    fn endpoint_invalid_hostname_rejected() {
        // Arrange
        let mut report = ValidationReport::default();
        let endpoint = EndpointSpec {
            host: HostSpec::Hostname("-invalid".to_string()),
            port: 3000,
            tls: None,
        };
        let origin = HclOrigin::test("endpoint");

        // Act
        endpoint.validate(&origin, &mut report);

        // Assert
        let error = report
            .errors()
            .first()
            .expect("expected at least one error");
        assert!(
            error
                .message
                .contains("invalid upstream hostname: -invalid")
        );
    }

    #[test]
    fn endpoint_port_zero_rejected() {
        // Arrange
        let mut report = ValidationReport::default();
        let endpoint = EndpointSpec {
            host: HostSpec::Ip(IpAddr::from_str("127.0.0.1").unwrap()),
            port: 0,
            tls: None,
        };
        let origin = HclOrigin::test("endpoint");

        // Act
        endpoint.validate(&origin, &mut report);

        // Assert
        let error = report
            .errors()
            .first()
            .expect("expected at least one error");
        assert!(error.message.contains("invalid port: 0"));
    }

    #[test]
    fn valid_endpoint() {
        // Arrange
        let mut report = ValidationReport::default();
        let endpoint = EndpointSpec {
            host: HostSpec::Ip(IpAddr::from_str("127.0.0.1").unwrap()),
            port: 3000,
            tls: None,
        };
        let origin = HclOrigin::test("endpoint");

        // Act
        endpoint.validate(&origin, &mut report);

        // Assert
        assert!(!report.has_issues());
    }

    #[test]
    fn endpoint_tls_sni_empty_rejected() {
        // Arrange
        let mut report = ValidationReport::default();
        let tls = EndpointTlsSpec {
            sni: "".to_string(),
            verify: false,
            ca_file: None,
        };
        let origin = HclOrigin::test("tls");

        // Act
        tls.validate(&origin, &mut report);

        // Assert
        let error = report
            .errors()
            .first()
            .expect("expected at least one error");
        assert!(error.message.contains("upstream TLS SNI required"));
    }

    #[test]
    fn endpoint_tls_sni_whitespace_rejected() {
        // Arrange
        let mut report = ValidationReport::default();
        let tls = EndpointTlsSpec {
            sni: "  ".to_string(),
            verify: false,
            ca_file: None,
        };
        let origin = HclOrigin::test("tls");

        // Act
        tls.validate(&origin, &mut report);

        // Assert
        let error = report
            .errors()
            .first()
            .expect("expected at least one error");
        assert!(error.message.contains("upstream TLS SNI required"));
    }
}
