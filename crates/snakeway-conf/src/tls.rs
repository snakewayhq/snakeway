use rustls_pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};

/// Parse PEM-encoded certificates from raw bytes.
///
/// Each certificate is validated as well-formed X.509 DER after decoding.
/// Returns an error if the PEM contains no certificates or if any
/// certificate has invalid DER content.
pub fn parse_cert_chain(bytes: &[u8]) -> Result<Vec<CertificateDer<'static>>, String> {
    let certs: Vec<_> = CertificateDer::pem_slice_iter(bytes)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("invalid PEM: {e}"))?;

    if certs.is_empty() {
        return Err("no certificates found in PEM".to_string());
    }

    for (i, cert_der) in certs.iter().enumerate() {
        x509_parser::parse_x509_certificate(cert_der.as_ref())
            .map_err(|e| format!("invalid X.509 certificate at index {i}: {e}"))?;
    }

    Ok(certs)
}

/// Parse a PEM-encoded private key from raw bytes.
///
/// Accepts PKCS#8, PKCS#1 (RSA), and SEC1 (EC) key formats.
/// Encrypted PEM keys are not supported.
pub fn parse_private_key(bytes: &[u8]) -> Result<PrivateKeyDer<'static>, String> {
    PrivateKeyDer::from_pem_slice(bytes).map_err(|e| format!("invalid private key PEM: {e}"))
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
) -> Result<pingora_rustls::sign::CertifiedKey, CertKeyError> {
    pingora_rustls::install_default_crypto_provider();
    let provider = pingora_rustls::CryptoProvider::get_default()
        .ok_or_else(|| CertKeyError::Other("TLS crypto provider is not available".to_string()))?;

    pingora_rustls::sign::CertifiedKey::from_der(certs, key, provider).map_err(|e| match e {
        pingora_rustls::RusTlsError::InconsistentKeys(_) => CertKeyError::KeyMismatch,
        other => CertKeyError::Other(other.to_string()),
    })
}

/// Distinguishes a key/cert mismatch from other failures when building a
/// `CertifiedKey`.
#[derive(Debug)]
pub enum CertKeyError {
    KeyMismatch,
    Other(String),
}

impl std::fmt::Display for CertKeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CertKeyError::KeyMismatch => write!(f, "private key does not match certificate"),
            CertKeyError::Other(msg) => write!(f, "{msg}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RSA_CERT: &str = include_str!("../fixtures/tls/rsa.pem");
    const RSA_PKCS1_KEY: &str = include_str!("../fixtures/tls/rsa-pkcs1.key");
    const EC_CERT: &str = include_str!("../fixtures/tls/ec.pem");
    const EC_SEC1_KEY: &str = include_str!("../fixtures/tls/ec-sec1.key");
    const EC_ENCRYPTED_PKCS8_KEY: &str = include_str!("../fixtures/tls/ec-encrypted-pkcs8.key");
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
        assert_eq!(
            result.expect_err("an encrypted key must be rejected"),
            "invalid private key PEM: no items found"
        );
    }

    #[test]
    fn parse_private_key_rejects_file_with_only_a_certificate() {
        // Arrange
        let bytes = EC_CERT.as_bytes();

        // Act
        let result = parse_private_key(bytes);

        // Assert
        assert_eq!(
            result.expect_err("a certificate is not a private key"),
            "invalid private key PEM: no items found"
        );
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
        let msg = result.expect_err("a bad intermediate must be rejected");
        assert!(
            msg.starts_with("invalid X.509 certificate at index 1: "),
            "got: {msg}"
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
            matches!(result, Err(CertKeyError::KeyMismatch)),
            "got: {result:?}"
        );
    }
}
