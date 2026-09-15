use pingora_rustls::RusTlsError;
use rustls_pki_types::pem::{PemObject, SectionKind};
use rustls_pki_types::{CertificateDer, PrivateKeyDer};

/// Parse PEM-encoded certificates from raw bytes.
///
/// Each certificate is validated as well-formed X.509 DER after decoding.
/// Sections of other kinds, such as a private key after the certificate, are skipped.
/// Returns an error if the PEM contains no certificates or if any
/// certificate has invalid DER content.
pub fn parse_cert_chain(bytes: &[u8]) -> Result<Vec<CertificateDer<'static>>, TlsError> {
    let certs: Vec<_> = CertificateDer::pem_slice_iter(bytes)
        .collect::<Result<Vec<_>, _>>()
        .map_err(TlsError::InvalidPem)?;

    check_certificates(certs)
}

/// Parse a PEM file of CA certificates from raw bytes.
///
/// Pingora's upstream connector refuses a CA file that holds a private key, a public key,
/// a certificate revocation list, or a certificate request. This function returns an error
/// for those sections before the file reaches the connector. Sections with any other label
/// are skipped, the same as in the connector.
pub fn parse_ca_certificates(bytes: &[u8]) -> Result<Vec<CertificateDer<'static>>, TlsError> {
    let mut certs = Vec::new();

    for section in <(SectionKind, Vec<u8>) as PemObject>::pem_slice_iter(bytes) {
        let (kind, der) = section.map_err(TlsError::InvalidPem)?;
        let label = match kind {
            SectionKind::Certificate => {
                certs.push(CertificateDer::from(der));
                continue;
            }
            SectionKind::PublicKey => "PUBLIC KEY",
            SectionKind::RsaPrivateKey => "RSA PRIVATE KEY",
            SectionKind::PrivateKey => "PRIVATE KEY",
            SectionKind::EcPrivateKey => "EC PRIVATE KEY",
            SectionKind::Crl => "X509 CRL",
            SectionKind::Csr => "CERTIFICATE REQUEST",
            _ => continue,
        };
        return Err(TlsError::NonCertificateSection { label });
    }

    check_certificates(certs)
}

fn check_certificates(
    certs: Vec<CertificateDer<'static>>,
) -> Result<Vec<CertificateDer<'static>>, TlsError> {
    if certs.is_empty() {
        return Err(TlsError::NoCertificates);
    }

    for (index, cert_der) in certs.iter().enumerate() {
        x509_parser::parse_x509_certificate(cert_der.as_ref()).map_err(|e| {
            TlsError::InvalidCertificate {
                index,
                reason: e.to_string(),
            }
        })?;
    }

    Ok(certs)
}

/// Parse a PEM-encoded private key from raw bytes.
///
/// Accepts PKCS#8, PKCS#1 (RSA), and SEC1 (EC) key formats.
/// Encrypted PEM keys are not supported.
pub fn parse_private_key(bytes: &[u8]) -> Result<PrivateKeyDer<'static>, TlsError> {
    PrivateKeyDer::from_pem_slice(bytes).map_err(TlsError::InvalidPrivateKeyPem)
}

/// Build a `CertifiedKey` from a parsed certificate chain and private key.
///
/// Verifies the private key matches the leaf certificate.
/// `from_der` silently accepts `InconsistentKeys::Unknown` when the key type
/// does not support SPKI comparison (exotic algorithms only).
///
/// If no process-level rustls crypto provider is installed yet, this installs
/// Pingora's default provider first, so you can call it before any listener or
/// connector exists.
pub fn build_certified_key(
    certs: Vec<CertificateDer<'static>>,
    key: PrivateKeyDer<'static>,
) -> Result<pingora_rustls::sign::CertifiedKey, TlsError> {
    pingora_rustls::install_default_crypto_provider();
    let provider =
        pingora_rustls::CryptoProvider::get_default().ok_or(TlsError::ProviderUnavailable)?;

    pingora_rustls::sign::CertifiedKey::from_der(certs, key, provider).map_err(|e| match e {
        RusTlsError::InconsistentKeys(_) => TlsError::KeyMismatch,
        RusTlsError::NoCertificatesPresented => TlsError::NoCertificates,
        rejected @ RusTlsError::InvalidCertificate(_) => TlsError::CertificateRejected(rejected),
        other => TlsError::UnsupportedPrivateKey(other),
    })
}

/// An error from reading PEM certificates or a private key, or from pairing them.
///
/// Certificate errors and private key errors are separate variants, so you can report
/// the file that caused the error.
#[derive(Debug, thiserror::Error)]
pub enum TlsError {
    #[error("invalid PEM: {0}")]
    InvalidPem(#[source] rustls_pki_types::pem::Error),

    #[error("no certificates found in PEM")]
    NoCertificates,

    #[error("invalid X.509 certificate at index {index}: {reason}")]
    InvalidCertificate { index: usize, reason: String },

    #[error("a CA file must contain only certificates, but it contains a {label} section")]
    NonCertificateSection { label: &'static str },

    #[error("certificate rejected: {0}")]
    CertificateRejected(#[source] RusTlsError),

    #[error("invalid private key PEM: {0}")]
    InvalidPrivateKeyPem(#[source] rustls_pki_types::pem::Error),

    #[error("unsupported private key: {0}")]
    UnsupportedPrivateKey(#[source] RusTlsError),

    #[error("private key does not match certificate")]
    KeyMismatch,

    #[error("TLS crypto provider is not available")]
    ProviderUnavailable,
}

#[cfg(test)]
mod tests {
    use super::*;

    const RSA_CERT: &str = include_str!("../fixtures/tls/rsa.pem");
    const RSA_PKCS1_KEY: &str = include_str!("../fixtures/tls/rsa-pkcs1.key");
    const EC_CERT: &str = include_str!("../fixtures/tls/ec.pem");
    const EC_SEC1_KEY: &str = include_str!("../fixtures/tls/ec-sec1.key");
    const EC_ENCRYPTED_PKCS8_KEY: &str = include_str!("../fixtures/tls/ec-encrypted-pkcs8.key");
    const EC_UNKNOWN_CRITICAL_EXTENSION_CERT: &str =
        include_str!("../fixtures/tls/ec-unknown-critical-extension.pem");
    const EC_P521_SEC1_KEY: &str = include_str!("../fixtures/tls/ec-p521-sec1.key");
    const BAD_CERT_BLOCK: &str =
        "-----BEGIN CERTIFICATE-----\naW52YWxpZA==\n-----END CERTIFICATE-----\n";

    fn der_of(pem: &str) -> Vec<u8> {
        let (_, parsed) =
            x509_parser::pem::parse_x509_pem(pem.as_bytes()).expect("fixture must be PEM");
        parsed.contents
    }

    #[test]
    fn parse_private_key_accepts_pkcs1_rsa_key() {
        // Arrange
        let bytes = RSA_PKCS1_KEY.as_bytes();

        // Act
        let result = parse_private_key(bytes);

        // Assert
        assert!(
            matches!(result, Ok(PrivateKeyDer::Pkcs1(_))),
            "got: {result:?}"
        );
    }

    #[test]
    fn parse_private_key_accepts_sec1_ec_key() {
        // Arrange
        let bytes = EC_SEC1_KEY.as_bytes();

        // Act
        let result = parse_private_key(bytes);

        // Assert
        assert!(
            matches!(result, Ok(PrivateKeyDer::Sec1(_))),
            "got: {result:?}"
        );
    }

    #[test]
    fn parse_private_key_accepts_pkcs8_key() {
        // Arrange
        let generated = rcgen::generate_simple_self_signed(vec!["localhost".into()])
            .expect("failed to generate self-signed cert");
        let key_pem = generated.signing_key.serialize_pem();

        // Act
        let result = parse_private_key(key_pem.as_bytes());

        // Assert
        assert!(
            matches!(result, Ok(PrivateKeyDer::Pkcs8(_))),
            "got: {result:?}"
        );
    }

    #[test]
    fn parse_private_key_rejects_encrypted_pkcs8_key() {
        // Arrange
        let bytes = EC_ENCRYPTED_PKCS8_KEY.as_bytes();

        // Act
        let result = parse_private_key(bytes);

        // Assert
        let err = result.expect_err("an encrypted key must be rejected");
        assert!(
            matches!(err, TlsError::InvalidPrivateKeyPem(_)),
            "got: {err:?}"
        );
        assert_eq!(err.to_string(), "invalid private key PEM: no items found");
    }

    #[test]
    fn parse_private_key_rejects_file_with_only_a_certificate() {
        // Arrange
        let bytes = EC_CERT.as_bytes();

        // Act
        let result = parse_private_key(bytes);

        // Assert
        let err = result.expect_err("a certificate is not a private key");
        assert!(
            matches!(err, TlsError::InvalidPrivateKeyPem(_)),
            "got: {err:?}"
        );
        assert_eq!(err.to_string(), "invalid private key PEM: no items found");
    }

    #[test]
    fn parse_private_key_reads_key_that_follows_a_certificate() {
        // Arrange
        let combined = format!("{EC_CERT}{EC_SEC1_KEY}");

        // Act
        let result = parse_private_key(combined.as_bytes());

        // Assert
        assert!(
            matches!(result, Ok(PrivateKeyDer::Sec1(_))),
            "got: {result:?}"
        );
    }

    #[test]
    fn parse_cert_chain_ignores_key_that_follows_a_certificate() {
        // Arrange
        let combined = format!("{EC_CERT}{EC_SEC1_KEY}");

        // Act
        let result = parse_cert_chain(combined.as_bytes());

        // Assert
        let certs = result.expect("the certificate must parse");
        assert_eq!(certs.len(), 1);
        assert_eq!(certs[0].as_ref(), der_of(EC_CERT).as_slice());
    }

    #[test]
    fn parse_cert_chain_returns_every_certificate_in_order() {
        // Arrange
        let chain = format!("{RSA_CERT}{EC_CERT}");

        // Act
        let result = parse_cert_chain(chain.as_bytes());

        // Assert
        let certs = result.expect("both certificates must parse");
        assert_eq!(certs.len(), 2);
        assert_eq!(certs[0].as_ref(), der_of(RSA_CERT).as_slice());
        assert_eq!(certs[1].as_ref(), der_of(EC_CERT).as_slice());
    }

    #[test]
    fn parse_cert_chain_reports_invalid_certificate_after_valid_leaf() {
        // Arrange
        let chain = format!("{EC_CERT}{BAD_CERT_BLOCK}");

        // Act
        let result = parse_cert_chain(chain.as_bytes());

        // Assert
        let err = result.expect_err("a bad intermediate must be rejected");
        assert!(
            matches!(err, TlsError::InvalidCertificate { index: 1, .. }),
            "got: {err:?}"
        );
        assert!(
            err.to_string()
                .starts_with("invalid X.509 certificate at index 1: "),
            "got: {err}"
        );
    }

    fn ca_bundle_with_section(label: &str) -> String {
        format!("{EC_CERT}-----BEGIN {label}-----\naW52YWxpZA==\n-----END {label}-----\n")
    }

    #[test]
    fn parse_ca_certificates_returns_every_certificate_in_order() {
        // Arrange
        let bundle = format!("{RSA_CERT}{EC_CERT}");

        // Act
        let result = parse_ca_certificates(bundle.as_bytes());

        // Assert
        let certs = result.expect("a bundle of certificates must parse");
        assert_eq!(certs.len(), 2);
        assert_eq!(certs[0].as_ref(), der_of(RSA_CERT).as_slice());
        assert_eq!(certs[1].as_ref(), der_of(EC_CERT).as_slice());
    }

    #[test]
    fn parse_ca_certificates_skips_sections_with_other_labels() {
        // Arrange
        let bundle = ca_bundle_with_section("SNAKEWAY TEST DATA");

        // Act
        let result = parse_ca_certificates(bundle.as_bytes());

        // Assert
        let certs = result.expect("a section with another label must be skipped");
        assert_eq!(certs.len(), 1);
        assert_eq!(certs[0].as_ref(), der_of(EC_CERT).as_slice());
    }

    #[test]
    fn parse_ca_certificates_rejects_private_key_section() {
        // Arrange
        let bundle = ca_bundle_with_section("PRIVATE KEY");

        // Act
        let result = parse_ca_certificates(bundle.as_bytes());

        // Assert
        assert!(
            matches!(
                result,
                Err(TlsError::NonCertificateSection {
                    label: "PRIVATE KEY"
                })
            ),
            "got: {result:?}"
        );
    }

    #[test]
    fn parse_ca_certificates_rejects_rsa_private_key_section() {
        // Arrange
        let bundle = ca_bundle_with_section("RSA PRIVATE KEY");

        // Act
        let result = parse_ca_certificates(bundle.as_bytes());

        // Assert
        assert!(
            matches!(
                result,
                Err(TlsError::NonCertificateSection {
                    label: "RSA PRIVATE KEY"
                })
            ),
            "got: {result:?}"
        );
    }

    #[test]
    fn parse_ca_certificates_rejects_ec_private_key_section() {
        // Arrange
        let bundle = ca_bundle_with_section("EC PRIVATE KEY");

        // Act
        let result = parse_ca_certificates(bundle.as_bytes());

        // Assert
        assert!(
            matches!(
                result,
                Err(TlsError::NonCertificateSection {
                    label: "EC PRIVATE KEY"
                })
            ),
            "got: {result:?}"
        );
    }

    #[test]
    fn parse_ca_certificates_rejects_public_key_section() {
        // Arrange
        let bundle = ca_bundle_with_section("PUBLIC KEY");

        // Act
        let result = parse_ca_certificates(bundle.as_bytes());

        // Assert
        assert!(
            matches!(
                result,
                Err(TlsError::NonCertificateSection {
                    label: "PUBLIC KEY"
                })
            ),
            "got: {result:?}"
        );
    }

    #[test]
    fn parse_ca_certificates_rejects_crl_section() {
        // Arrange
        let bundle = ca_bundle_with_section("X509 CRL");

        // Act
        let result = parse_ca_certificates(bundle.as_bytes());

        // Assert
        assert!(
            matches!(
                result,
                Err(TlsError::NonCertificateSection { label: "X509 CRL" })
            ),
            "got: {result:?}"
        );
    }

    #[test]
    fn parse_ca_certificates_rejects_certificate_request_section() {
        // Arrange
        let bundle = ca_bundle_with_section("CERTIFICATE REQUEST");

        // Act
        let result = parse_ca_certificates(bundle.as_bytes());

        // Assert
        assert!(
            matches!(
                result,
                Err(TlsError::NonCertificateSection {
                    label: "CERTIFICATE REQUEST"
                })
            ),
            "got: {result:?}"
        );
    }

    #[test]
    fn build_certified_key_accepts_pkcs1_rsa_pair() {
        // Arrange
        let certs = parse_cert_chain(RSA_CERT.as_bytes()).expect("fixture cert must parse");
        let key = parse_private_key(RSA_PKCS1_KEY.as_bytes()).expect("fixture key must parse");

        // Act
        let result = build_certified_key(certs, key);

        // Assert
        let certified = result.expect("a PKCS#1 RSA pair must build");
        assert_eq!(certified.cert.len(), 1);
        assert_eq!(certified.cert[0].as_ref(), der_of(RSA_CERT).as_slice());
    }

    #[test]
    fn build_certified_key_accepts_sec1_ec_pair() {
        // Arrange
        let certs = parse_cert_chain(EC_CERT.as_bytes()).expect("fixture cert must parse");
        let key = parse_private_key(EC_SEC1_KEY.as_bytes()).expect("fixture key must parse");

        // Act
        let result = build_certified_key(certs, key);

        // Assert
        let certified = result.expect("a SEC1 EC pair must build");
        assert_eq!(certified.cert.len(), 1);
        assert_eq!(certified.cert[0].as_ref(), der_of(EC_CERT).as_slice());
    }

    #[test]
    fn build_certified_key_reports_mismatch_between_rsa_cert_and_ec_key() {
        // Arrange
        let certs = parse_cert_chain(RSA_CERT.as_bytes()).expect("fixture cert must parse");
        let key = parse_private_key(EC_SEC1_KEY.as_bytes()).expect("fixture key must parse");

        // Act
        let result = build_certified_key(certs, key);

        // Assert
        assert!(
            matches!(result, Err(TlsError::KeyMismatch)),
            "got: {result:?}"
        );
    }

    #[test]
    fn build_certified_key_reports_certificate_that_rustls_rejects() {
        // Arrange
        let certs = parse_cert_chain(EC_UNKNOWN_CRITICAL_EXTENSION_CERT.as_bytes())
            .expect("x509-parser must accept the fixture cert");
        let key = parse_private_key(EC_SEC1_KEY.as_bytes()).expect("fixture key must parse");

        // Act
        let result = build_certified_key(certs, key);

        // Assert
        assert!(
            matches!(
                result,
                Err(TlsError::CertificateRejected(
                    RusTlsError::InvalidCertificate(_)
                ))
            ),
            "got: {result:?}"
        );
    }

    #[test]
    fn build_certified_key_reports_private_key_that_provider_cannot_load() {
        // Arrange
        let certs = parse_cert_chain(EC_CERT.as_bytes()).expect("fixture cert must parse");
        let key = parse_private_key(EC_P521_SEC1_KEY.as_bytes()).expect("fixture key must parse");

        // Act
        let result = build_certified_key(certs, key);

        // Assert
        assert!(
            matches!(result, Err(TlsError::UnsupportedPrivateKey(_))),
            "got: {result:?}"
        );
    }

    #[test]
    fn build_certified_key_reports_empty_chain() {
        // Arrange
        let key = parse_private_key(EC_SEC1_KEY.as_bytes()).expect("fixture key must parse");

        // Act
        let result = build_certified_key(Vec::new(), key);

        // Assert
        assert!(
            matches!(result, Err(TlsError::NoCertificates)),
            "got: {result:?}"
        );
    }
}
