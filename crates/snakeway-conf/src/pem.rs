use pingora_rustls::CertificateDer;
use std::io::Cursor;

/// Parse PEM-encoded certificates from raw bytes.
///
/// Each certificate is validated as well-formed X.509 DER after decoding.
/// Returns an error if the PEM contains no certificates or if any
/// certificate has invalid DER content.
pub fn parse_cert_chain(bytes: &[u8]) -> Result<Vec<CertificateDer<'static>>, String> {
    let certs: Vec<_> = rustls_pemfile::certs(&mut Cursor::new(bytes))
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
pub fn parse_private_key(
    bytes: &[u8],
) -> Result<pingora_rustls::PrivateKeyDer<'static>, String> {
    rustls_pemfile::private_key(&mut Cursor::new(bytes))
        .map_err(|e| format!("invalid private key PEM: {e}"))?
        .ok_or_else(|| "no private key found in PEM".to_string())
}
