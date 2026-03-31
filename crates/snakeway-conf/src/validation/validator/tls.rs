use crate::validation::validator::read_nonempty_file;
use openssl::pkey::{PKey, Private};
use openssl::x509::X509;
use std::path::Path;

pub(crate) fn validate_cert_pem(path: &Path) -> Result<(), String> {
    let bytes = read_nonempty_file(path)?;

    X509::stack_from_pem(&bytes)
        .map_err(|e| format!("invalid PEM certificate {}: {e}", path.display()))?;

    Ok(())
}

pub(crate) fn validate_cert_key_pair(cert_path: &Path, key_path: &Path) -> Result<(), String> {
    let cert_bytes = read_nonempty_file(cert_path)?;
    let key_bytes = read_nonempty_file(key_path)?;

    let mut chain = X509::stack_from_pem(&cert_bytes)
        .map_err(|e| format!("invalid certificate PEM {}: {e}", cert_path.display()))?;

    if chain.is_empty() {
        return Err(format!(
            "certificate file contains no certificates: {}",
            cert_path.display()
        ));
    }

    let leaf = chain.remove(0);

    let key = PKey::<Private>::private_key_from_pem(&key_bytes)
        .map_err(|e| format!("invalid private key PEM {}: {e}", key_path.display()))?;

    let public_key = leaf.public_key().map_err(|e| {
        format!(
            "cannot extract public key from cert {}: {e}",
            cert_path.display()
        )
    })?;

    if !public_key.public_eq(&key) {
        return Err(format!(
            "private key does not match certificate: cert={}, key={}",
            cert_path.display(),
            key_path.display()
        ));
    }

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
            b"-----BEGIN CERTIFICATE-----\ninvalid base64 content\n-----END CERTIFICATE-----\n",
        )
        .expect("failed to write");

        // Act
        let result = validate_cert_pem(&path);

        // Assert
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid PEM"));
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
        assert!(result.unwrap_err().contains("invalid private key"));
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
