use crate::pem;
use crate::validation::validator::read_nonempty_file;
use pingora_rustls::{CryptoProvider, sign};
use std::path::Path;

pub(crate) fn validate_cert_pem(path: &Path) -> Result<(), String> {
    let bytes = read_nonempty_file(path)?;

    pem::parse_cert_chain(&bytes)
        .map_err(|e| format!("{}: {e}", path.display()))?;

    Ok(())
}

pub(crate) fn validate_cert_key_pair(cert_path: &Path, key_path: &Path) -> Result<(), String> {
    let cert_bytes = read_nonempty_file(cert_path)?;
    let key_bytes = read_nonempty_file(key_path)?;

    let certs = pem::parse_cert_chain(&cert_bytes)
        .map_err(|e| format!("{}: {e}", cert_path.display()))?;

    let key = pem::parse_private_key(&key_bytes)
        .map_err(|e| format!("{}: {e}", key_path.display()))?;

    let provider = CryptoProvider::get_default().ok_or_else(|| {
        "TLS crypto provider not installed (call install_default_crypto_provider at startup)"
            .to_string()
    })?;

    // from_der silently accepts InconsistentKeys::Unknown when the key type
    // does not support SPKI comparison (exotic algorithms only).
    sign::CertifiedKey::from_der(certs, key, provider).map_err(|e| match e {
        pingora_rustls::RusTlsError::InconsistentKeys(_) => {
            format!(
                "private key does not match certificate: cert={}, key={}",
                cert_path.display(),
                key_path.display()
            )
        }
        other => {
            format!(
                "failed to load private key for {}: {other}",
                key_path.display()
            )
        }
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rcgen::generate_simple_self_signed;
    use std::fs::File;
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn validate_cert_pem_file_not_found() {
        // Arrange
        let path = PathBuf::from("/nonexistent/cert.pem");

        // Act
        let result = validate_cert_pem(&path);

        // Assert
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("file does not exist"));
    }

    #[test]
    fn validate_cert_pem_invalid_content() {
        // Arrange
        let dir = tempdir().expect("failed to create temp dir");
        let path = dir.path().join("bad_cert.pem");
        let mut f = File::create(&path).expect("failed to create file");
        f.write_all(
            b"-----BEGIN CERTIFICATE-----\naW52YWxpZA==\n-----END CERTIFICATE-----\n",
        )
        .expect("failed to write");

        // Act
        let result = validate_cert_pem(&path);

        // Assert
        assert!(result.is_err());
        assert!(
            result.unwrap_err().contains("invalid X.509 certificate at index 0"),
            "expected an X.509 DER validation error"
        );
    }

    #[test]
    fn validate_cert_pem_valid() {
        // Arrange
        let dir = tempdir().expect("failed to create temp dir");
        let cert = generate_simple_self_signed(vec!["localhost".into()])
            .expect("failed to generate self-signed cert");
        let cert_pem = cert.cert.pem();
        let path = dir.path().join("cert.pem");
        let mut f = File::create(&path).expect("failed to create file");
        f.write_all(cert_pem.as_bytes()).expect("failed to write");

        // Act
        let result = validate_cert_pem(&path);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn validate_cert_key_pair_valid() {
        // Arrange
        pingora_rustls::install_default_crypto_provider();
        let dir = tempdir().expect("failed to create temp dir");
        let cert = generate_simple_self_signed(vec!["localhost".into()])
            .expect("failed to generate self-signed cert");
        let cert_pem = cert.cert.pem();
        let key_pem = cert.signing_key.serialize_pem();

        let cert_path = dir.path().join("cert.pem");
        let mut cf = File::create(&cert_path).expect("failed to create cert file");
        cf.write_all(cert_pem.as_bytes())
            .expect("failed to write cert");

        let key_path = dir.path().join("key.pem");
        let mut kf = File::create(&key_path).expect("failed to create key file");
        kf.write_all(key_pem.as_bytes())
            .expect("failed to write key");

        // Act
        let result = validate_cert_key_pair(&cert_path, &key_path);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn validate_cert_key_pair_key_mismatch() {
        // Arrange
        pingora_rustls::install_default_crypto_provider();
        let dir = tempdir().expect("failed to create temp dir");

        let cert1 = generate_simple_self_signed(vec!["first.localhost".into()])
            .expect("failed to generate first cert");
        let cert2 = generate_simple_self_signed(vec!["second.localhost".into()])
            .expect("failed to generate second cert");

        let cert_path = dir.path().join("cert.pem");
        let mut cf = File::create(&cert_path).expect("failed to create cert file");
        cf.write_all(cert1.cert.pem().as_bytes())
            .expect("failed to write cert");

        let key_path = dir.path().join("key.pem");
        let mut kf = File::create(&key_path).expect("failed to create key file");
        kf.write_all(cert2.signing_key.serialize_pem().as_bytes())
            .expect("failed to write key");

        // Act
        let result = validate_cert_key_pair(&cert_path, &key_path);

        // Assert
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not match"));
    }

    #[test]
    fn validate_cert_key_pair_not_a_key() {
        // Arrange
        let dir = tempdir().expect("failed to create temp dir");
        let cert = generate_simple_self_signed(vec!["localhost".into()])
            .expect("failed to generate self-signed cert");

        let cert_path = dir.path().join("cert.pem");
        let mut cf = File::create(&cert_path).expect("failed to create cert file");
        cf.write_all(cert.cert.pem().as_bytes())
            .expect("failed to write cert");

        let key_path = dir.path().join("key.pem");
        let mut kf = File::create(&key_path).expect("failed to create key file");
        kf.write_all(b"not a key").expect("failed to write key");

        // Act
        let result = validate_cert_key_pair(&cert_path, &key_path);

        // Assert
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no private key found"));
    }

    #[test]
    fn validate_cert_key_pair_empty_cert_file() {
        // Arrange
        let dir = tempdir().expect("failed to create temp dir");

        let cert_path = dir.path().join("cert.pem");
        File::create(&cert_path).expect("failed to create empty cert file");

        let key_path = dir.path().join("key.pem");
        let mut kf = File::create(&key_path).expect("failed to create key file");
        kf.write_all(b"some key content")
            .expect("failed to write key");

        // Act
        let result = validate_cert_key_pair(&cert_path, &key_path);

        // Assert
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty"));
    }
}
