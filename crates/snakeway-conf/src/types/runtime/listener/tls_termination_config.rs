use crate::types::{ACME_CHALLENGE_HTTP01, TlsTerminationSpec};
use confval::provenance::{Lower, Report};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Paths are validated and resolved during config validation.
/// Runtime code assumes these values are valid.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum TlsTerminationConfig {
    Manual {
        cert: PathBuf,
        key: PathBuf,
    },
    Acme {
        domains: Vec<String>,
        challenge: AcmeChallengeConfig,
    },
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum AcmeChallengeConfig {
    Http01,
}

impl Lower<TlsTerminationSpec> for TlsTerminationConfig {
    fn lower(spec: &TlsTerminationSpec, report: &mut Report) -> Option<Self> {
        match spec {
            TlsTerminationSpec::Manual { cert, key } => Some(TlsTerminationConfig::Manual {
                cert: cert.value.clone(),
                key: key.value.clone(),
            }),

            TlsTerminationSpec::Acme { domains, challenge } => {
                let challenge = match challenge.value.as_str() {
                    ACME_CHALLENGE_HTTP01 => AcmeChallengeConfig::Http01,
                    other => {
                        report
                            .error(format!("unknown ACME challenge: {other}"))
                            .at(challenge.span)
                            .emit();
                        return None;
                    }
                };

                let mut canonicalize_domains: Vec<String> = domains
                    .iter()
                    .map(|d| d.value.trim().to_ascii_lowercase())
                    .filter(|d| !d.is_empty())
                    .collect();

                canonicalize_domains.sort();
                canonicalize_domains.dedup();

                Some(TlsTerminationConfig::Acme {
                    domains: canonicalize_domains,
                    challenge,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use confval::provenance::Located;

    fn acme(domains: Vec<&str>) -> TlsTerminationSpec {
        TlsTerminationSpec::Acme {
            domains: domains
                .into_iter()
                .map(|d| Located::detached(d.to_string()))
                .collect(),
            challenge: Located::detached(ACME_CHALLENGE_HTTP01.to_string()),
        }
    }

    fn lower(spec: &TlsTerminationSpec) -> TlsTerminationConfig {
        let mut report = Report::new();
        let config = TlsTerminationConfig::lower(spec, &mut report);
        assert!(!report.has_errors(), "issues: {:?}", report.issues());
        config.unwrap()
    }

    #[test]
    fn acme_challenge_maps_http01() {
        // Arrange
        let spec = acme(vec!["example.com"]);

        // Act
        let config = lower(&spec);

        // Assert
        assert!(matches!(
            config,
            TlsTerminationConfig::Acme {
                challenge: AcmeChallengeConfig::Http01,
                ..
            }
        ));
    }

    #[test]
    fn manual_tls_passes_through() {
        // Arrange
        let spec = TlsTerminationSpec::Manual {
            cert: Located::detached(PathBuf::from("/etc/ssl/cert.pem")),
            key: Located::detached(PathBuf::from("/etc/ssl/key.pem")),
        };

        // Act
        let config = lower(&spec);

        // Assert
        match config {
            TlsTerminationConfig::Manual { cert, key } => {
                assert_eq!(cert, PathBuf::from("/etc/ssl/cert.pem"));
                assert_eq!(key, PathBuf::from("/etc/ssl/key.pem"));
            }
            _ => panic!("expected Manual variant"),
        }
    }

    #[test]
    fn acme_domains_trimmed_and_lowercased() {
        // Arrange
        let spec = acme(vec![" Example.COM "]);

        // Act
        let config = lower(&spec);

        // Assert
        match config {
            TlsTerminationConfig::Acme { domains, .. } => {
                assert_eq!(domains, vec!["example.com"]);
            }
            _ => panic!("expected Acme variant"),
        }
    }

    #[test]
    fn acme_domains_sorted_and_deduped() {
        // Arrange
        let spec = acme(vec!["b.com", "a.com", "b.com"]);

        // Act
        let config = lower(&spec);

        // Assert
        match config {
            TlsTerminationConfig::Acme { domains, .. } => {
                assert_eq!(domains, vec!["a.com", "b.com"]);
            }
            _ => panic!("expected Acme variant"),
        }
    }

    #[test]
    fn acme_empty_domains_filtered() {
        // Arrange
        let spec = acme(vec!["", "  ", "valid.com"]);

        // Act
        let config = lower(&spec);

        // Assert
        match config {
            TlsTerminationConfig::Acme { domains, .. } => {
                assert_eq!(domains, vec!["valid.com"]);
            }
            _ => panic!("expected Acme variant"),
        }
    }

    #[test]
    fn unknown_challenge_is_reported() {
        // Arrange
        let spec = TlsTerminationSpec::Acme {
            domains: vec![Located::detached("example.com".to_string())],
            challenge: Located::detached("dns01".to_string()),
        };

        // Act
        let mut report = Report::new();
        let config = TlsTerminationConfig::lower(&spec, &mut report);

        // Assert
        assert!(config.is_none());
        assert!(
            report
                .issues()
                .iter()
                .any(|i| i.message == "unknown ACME challenge: dns01")
        );
    }
}
