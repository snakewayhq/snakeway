use super::bind_issues;
use crate::types::{HclOrigin, NetworkConnectionFilterSpec};
use confval::{ValidateSpec, ValidationReport};

impl ValidateSpec<HclOrigin> for NetworkConnectionFilterSpec {
    fn validate(&self, origin: &HclOrigin, report: &mut ValidationReport<HclOrigin>) {
        if !self.ip_family.ipv4 && !self.ip_family.ipv6 {
            report.push(bind_issues::connection_filter_requires_at_least_one_ip_family(origin));
        }

        for cidr in &self.cidr.allow {
            if cidr.parse::<ipnet::IpNet>().is_err() {
                report.push(bind_issues::invalid_cidr_in_connection_filter_allow_list(
                    cidr, origin,
                ));
            }
        }

        for cidr in &self.cidr.deny {
            if cidr.parse::<ipnet::IpNet>().is_err() {
                report.push(bind_issues::invalid_cidr_in_connection_filter_deny_list(
                    cidr, origin,
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::types::{
        CidrSpec, HclOrigin, IpFamilySpec, NetworkConnectionFilterSpec, OnNoPeerAddrSpec,
    };
    use confval::{ValidateSpec, ValidationReport};

    fn test_origin() -> HclOrigin {
        HclOrigin::test("connection_filter")
    }

    #[test]
    fn requires_at_least_one_ip_family() {
        // Arrange
        let spec = NetworkConnectionFilterSpec {
            ip_family: IpFamilySpec {
                ipv4: false,
                ipv6: false,
            },
            cidr: CidrSpec::default(),
            on_no_peer_addr: OnNoPeerAddrSpec::default(),
        };
        let origin = test_origin();
        let mut report = ValidationReport::default();

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert_eq!(report.errors().len(), 1);
    }

    #[test]
    fn invalid_cidr_in_allow_list() {
        // Arrange
        let spec = NetworkConnectionFilterSpec {
            ip_family: IpFamilySpec {
                ipv4: true,
                ipv6: false,
            },
            cidr: CidrSpec {
                allow: vec!["not-a-cidr".to_string()],
                deny: vec![],
            },
            on_no_peer_addr: OnNoPeerAddrSpec::default(),
        };
        let origin = test_origin();
        let mut report = ValidationReport::default();

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert_eq!(report.errors().len(), 1);
    }

    #[test]
    fn invalid_cidr_in_deny_list() {
        // Arrange
        let spec = NetworkConnectionFilterSpec {
            ip_family: IpFamilySpec {
                ipv4: true,
                ipv6: false,
            },
            cidr: CidrSpec {
                allow: vec![],
                deny: vec!["not-a-cidr".to_string()],
            },
            on_no_peer_addr: OnNoPeerAddrSpec::default(),
        };
        let origin = test_origin();
        let mut report = ValidationReport::default();

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert_eq!(report.errors().len(), 1);
    }

    #[test]
    fn valid_connection_filter() {
        // Arrange
        let spec = NetworkConnectionFilterSpec {
            ip_family: IpFamilySpec {
                ipv4: true,
                ipv6: false,
            },
            cidr: CidrSpec {
                allow: vec!["192.168.1.0/24".to_string()],
                deny: vec![],
            },
            on_no_peer_addr: OnNoPeerAddrSpec::default(),
        };
        let origin = test_origin();
        let mut report = ValidationReport::default();

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert!(report.errors().is_empty());
    }
}
