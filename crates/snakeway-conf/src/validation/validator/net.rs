use crate::types::Origin;
use crate::validation::ValidationReportDeprecated;
use ipnet::IpNet;
use std::net::IpAddr;

/// Checks if a string is a valid hostname according to DNS rules.
/// Validates length constraints (max 253 chars total, max 63 per label),
/// alphanumeric/hyphen characters only, and proper hyphen placement.
pub(crate) fn is_valid_hostname(s: &str) -> bool {
    if s.is_empty() || s.len() > 253 {
        return false;
    }

    s.split('.').all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
            && !label.starts_with('-')
            && !label.ends_with('-')
    })
}

/// Checks if a port number is valid (must be greater than 0).
/// It is naturally bounded by the upper limit of u16 (65535).
pub(crate) const fn is_valid_port(port: u16) -> bool {
    port > 0
}

/// NOTE: This function identifies non-globally-routable infrastructure address
/// space (RFC1918, ULA, loopback, link-local).
/// It MUST NOT be used to determine the absolute trustworthiness of a peer.
fn is_non_public_infra_network(net: &IpNet) -> bool {
    match &net.addr() {
        IpAddr::V4(v4) => v4.is_private() || v4.is_loopback() || v4.is_link_local(),
        IpAddr::V6(v6) => v6.is_loopback() || v6.is_unique_local() || v6.is_unicast_link_local(),
    }
}

pub(crate) fn validate_trusted_proxies(
    proxies: &[String],
    report: &mut ValidationReportDeprecated,
    origin: &Origin,
) {
    let mut networks = Vec::new();
    for proxy in proxies {
        if let Ok(net) = proxy.parse::<IpNet>() {
            networks.push(net);
        } else {
            report.invalid_trusted_proxy(proxy, origin);
        }
    }

    for network in networks {
        if network.prefix_len() == 0 {
            report.trusted_proxies_cannot_trust_all_networks(origin);
        }

        if !is_non_public_infra_network(&network) {
            report.trusted_proxies_contains_a_public_ip_range_warning(network, origin);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hostname_empty_rejected() {
        // Arrange
        let input = "";

        // Act
        let result = is_valid_hostname(input);

        // Assert
        assert!(!result);
    }

    #[test]
    fn hostname_too_long_rejected() {
        // Arrange
        let input = "a".repeat(254);

        // Act
        let result = is_valid_hostname(&input);

        // Assert
        assert!(!result);
    }

    #[test]
    fn hostname_label_too_long_rejected() {
        // Arrange
        let long_label = "a".repeat(64);
        let input = format!("{long_label}.com");

        // Act
        let result = is_valid_hostname(&input);

        // Assert
        assert!(!result);
    }

    #[test]
    fn hostname_consecutive_dots_rejected() {
        // Arrange
        let input = "example..com";

        // Act
        let result = is_valid_hostname(input);

        // Assert
        assert!(!result);
    }

    #[test]
    fn hostname_leading_hyphen_rejected() {
        // Arrange
        let input = "-example.com";

        // Act
        let result = is_valid_hostname(input);

        // Assert
        assert!(!result);
    }

    #[test]
    fn hostname_trailing_hyphen_rejected() {
        // Arrange
        let input = "example-.com";

        // Act
        let result = is_valid_hostname(input);

        // Assert
        assert!(!result);
    }

    #[test]
    fn valid_hostname() {
        // Arrange
        let input = "example.com";

        // Act
        let result = is_valid_hostname(input);

        // Assert
        assert!(result);
    }

    #[test]
    fn valid_single_label() {
        // Arrange
        let input = "localhost";

        // Act
        let result = is_valid_hostname(input);

        // Assert
        assert!(result);
    }
}
