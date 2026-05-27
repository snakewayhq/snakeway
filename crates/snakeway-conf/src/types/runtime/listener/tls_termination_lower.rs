use crate::types::TlsTerminationSpec;

use super::TlsTerminationConfig;

impl TryFrom<TlsTerminationSpec> for TlsTerminationConfig {
    type Error = String;

    fn try_from(spec: TlsTerminationSpec) -> Result<Self, Self::Error> {
        match spec {
            TlsTerminationSpec::Manual { cert, key } => {
                Ok(TlsTerminationConfig::Manual { cert, key })
            }

            TlsTerminationSpec::Acme { domains, challenge } => {
                let mut canonicalize_domains: Vec<String> = domains
                    .iter()
                    .map(|d| d.trim().to_ascii_lowercase())
                    .filter(|d| !d.is_empty())
                    .collect();

                canonicalize_domains.sort();
                canonicalize_domains.dedup();

                Ok(TlsTerminationConfig::Acme {
                    domains: canonicalize_domains,
                    challenge: challenge.into(),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AcmeChallengeSpec;
    use std::path::PathBuf;

    #[test]
    fn manual_tls_passes_through() {
        // Arrange
        let spec = TlsTerminationSpec::Manual {
            cert: PathBuf::from("/etc/ssl/cert.pem"),
            key: PathBuf::from("/etc/ssl/key.pem"),
        };

        // Act
        let config = TlsTerminationConfig::try_from(spec).unwrap();

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
        let spec = TlsTerminationSpec::Acme {
            domains: vec![" Example.COM ".to_string()],
            challenge: AcmeChallengeSpec::Http01,
        };

        // Act
        let config = TlsTerminationConfig::try_from(spec).unwrap();

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
        let spec = TlsTerminationSpec::Acme {
            domains: vec![
                "b.com".to_string(),
                "a.com".to_string(),
                "b.com".to_string(),
            ],
            challenge: AcmeChallengeSpec::Http01,
        };

        // Act
        let config = TlsTerminationConfig::try_from(spec).unwrap();

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
        let spec = TlsTerminationSpec::Acme {
            domains: vec!["".to_string(), "  ".to_string(), "valid.com".to_string()],
            challenge: AcmeChallengeSpec::Http01,
        };

        // Act
        let config = TlsTerminationConfig::try_from(spec).unwrap();

        // Assert
        match config {
            TlsTerminationConfig::Acme { domains, .. } => {
                assert_eq!(domains, vec!["valid.com"]);
            }
            _ => panic!("expected Acme variant"),
        }
    }
}
