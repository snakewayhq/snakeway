use confval::prelude::{Located, Report};
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

/// Checks if a port number is valid (must be in the range 1..=65535).
/// The spec layer uses `i64` (`HclInt`) for all numeric fields, so
/// validation must also reject values outside the `u16` range.
pub(crate) const fn is_valid_port(port: i64) -> bool {
    port >= 1 && port <= 65535
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

pub(crate) fn validate_trusted_proxies(proxies: &[Located<String>], report: &mut Report) {
    let mut networks = Vec::new();
    for proxy in proxies {
        if let Ok(net) = proxy.value.parse::<IpNet>() {
            networks.push((net, proxy.span));
        } else {
            report
                .error(format!("invalid trusted proxy: {}", proxy.value))
                .at(proxy.span)
                .emit();
        }
    }

    for (network, span) in networks {
        if network.prefix_len() == 0 {
            report
                .error("trusted_proxies must not contain a catch-all network (0.0.0.0/0 or ::/0)")
                .at(span)
                .emit();
        }

        if !is_non_public_infra_network(&network) {
            report
                .warning(format!(
                    "trusted_proxies should NOT contain a public IP range: {network}"
                ))
                .at(span)
                .emit();
        }
    }
}

/// Remediation shown for any malformed CIDR. The guidance is identical
/// everywhere a CIDR is parsed, so it lives here rather than at each call site.
const CIDR_HELP: &str =
    "CIDR must be a valid IPv4 or IPv6 network (e.g., 10.0.0.0/8 or 2001:db8::/32).";

/// Parses a list of located CIDR strings into [`IpNet`] values for lowering.
/// Each entry that fails to parse is reported at its own span, with a help line
/// of [`CIDR_HELP`]; the whole list fails (`None`) if any entry is invalid.
/// `context` names the list in the error message, e.g. "connection filter allow
/// list" or "network policy allow list". This is the single authority for CIDR
/// parsing and reporting, used by both the validation and lowering phases.
pub(crate) fn parse_cidr_list(
    list: &[Located<String>],
    context: &str,
    report: &mut Report,
) -> Option<Vec<IpNet>> {
    let mut out = Vec::with_capacity(list.len());
    let mut ok = true;
    for entry in list {
        match entry.value.parse::<IpNet>() {
            Ok(net) => out.push(net),
            Err(e) => {
                report
                    .error(format!(
                        "invalid CIDR in {context} '{}': {}",
                        entry.value, e
                    ))
                    .at(entry.span)
                    .help(CIDR_HELP)
                    .emit();
                ok = false;
            }
        }
    }
    ok.then_some(out)
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
