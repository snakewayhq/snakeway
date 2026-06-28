use crate::validation::validator::parse_cidr_list;
use confval::prelude::{Located, Report};
use serde::Serialize;

pub const ON_NO_PEER_ADDR_ALLOW: &str = "allow";
pub const ON_NO_PEER_ADDR_DENY: &str = "deny";

#[derive(Debug, Serialize, Clone, confval::Spec)]
pub struct NetworkConnectionFilterSpec {
    #[confval(nested)]
    pub cidr: Located<CidrSpec>,
    #[confval(nested)]
    pub ip_family: Located<IpFamilySpec>,
    pub on_no_peer_addr: Located<String>,
}

#[derive(Debug, Serialize, Default, Clone, confval::Spec)]
pub struct CidrSpec {
    #[confval(default)]
    pub allow: Vec<Located<String>>,
    #[confval(default)]
    pub deny: Vec<Located<String>>,
}

#[derive(Debug, Serialize, Default, Clone, confval::Spec)]
pub struct IpFamilySpec {
    pub ipv4: Located<bool>,
    pub ipv6: Located<bool>,
}

impl Default for NetworkConnectionFilterSpec {
    fn default() -> Self {
        Self {
            cidr: Located::detached(CidrSpec::default()),
            ip_family: Located::detached(IpFamilySpec::default()),
            on_no_peer_addr: Located::detached(ON_NO_PEER_ADDR_ALLOW.to_string()),
        }
    }
}

pub(crate) fn validate_network_connection_filter(
    spec: &NetworkConnectionFilterSpec,
    report: &mut Report,
) {
    if !spec.ip_family.value.ipv4.value && !spec.ip_family.value.ipv6.value {
        report
            .error("connection_filter must enable at least one IP family")
            .at(spec.ip_family.span)
            .help("Set ip_family.ipv4 and/or ip_family.ipv6 to true.")
            .emit();
    }

    let _ = parse_cidr_list(
        &spec.cidr.value.allow,
        "connection filter allow list",
        report,
    );
    let _ = parse_cidr_list(&spec.cidr.value.deny, "connection filter deny list", report);

    if spec.on_no_peer_addr.value != ON_NO_PEER_ADDR_ALLOW
        && spec.on_no_peer_addr.value != ON_NO_PEER_ADDR_DENY
    {
        report
            .error(format!(
                "unknown on_no_peer_addr: {}",
                spec.on_no_peer_addr.value
            ))
            .at(spec.on_no_peer_addr.span)
            .help("expected \"allow\" or \"deny\"")
            .emit();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filter(
        allow: Vec<&str>,
        deny: Vec<&str>,
        ipv4: bool,
        ipv6: bool,
    ) -> NetworkConnectionFilterSpec {
        NetworkConnectionFilterSpec {
            cidr: Located::detached(CidrSpec {
                allow: allow
                    .into_iter()
                    .map(|c| Located::detached(c.to_string()))
                    .collect(),
                deny: deny
                    .into_iter()
                    .map(|c| Located::detached(c.to_string()))
                    .collect(),
            }),
            ip_family: Located::detached(IpFamilySpec {
                ipv4: Located::detached(ipv4),
                ipv6: Located::detached(ipv6),
            }),
            on_no_peer_addr: Located::detached(ON_NO_PEER_ADDR_ALLOW.to_string()),
        }
    }

    #[test]
    fn valid_filter_produces_no_errors() {
        // Arrange
        let spec = filter(vec!["10.0.0.0/8"], vec!["192.168.0.0/16"], true, false);
        let mut report = Report::new();

        // Act
        validate_network_connection_filter(&spec, &mut report);

        // Assert
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
    }

    #[test]
    fn no_ip_family_is_rejected() {
        // Arrange
        let spec = filter(vec![], vec![], false, false);
        let mut report = Report::new();

        // Act
        validate_network_connection_filter(&spec, &mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message == "connection_filter must enable at least one IP family")
        );
    }

    #[test]
    fn invalid_allow_cidr_is_rejected() {
        // Arrange
        let spec = filter(vec!["not a cidr"], vec![], true, true);
        let mut report = Report::new();

        // Act
        validate_network_connection_filter(&spec, &mut report);

        // Assert
        assert!(report.issues().iter().any(|e| {
            e.message
                .contains("invalid CIDR in connection filter allow list 'not a cidr'")
        }));
    }

    #[test]
    fn invalid_deny_cidr_is_rejected() {
        // Arrange
        let spec = filter(vec![], vec!["bad/99"], true, true);
        let mut report = Report::new();

        // Act
        validate_network_connection_filter(&spec, &mut report);

        // Assert
        assert!(report.issues().iter().any(|e| {
            e.message
                .contains("invalid CIDR in connection filter deny list 'bad/99'")
        }));
    }

    #[test]
    fn unknown_on_no_peer_addr_is_rejected() {
        // Arrange
        let mut spec = filter(vec![], vec![], true, true);
        spec.on_no_peer_addr = Located::detached("maybe".to_string());
        let mut report = Report::new();

        // Act
        validate_network_connection_filter(&spec, &mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message == "unknown on_no_peer_addr: maybe")
        );
    }
}
