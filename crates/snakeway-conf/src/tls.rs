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
