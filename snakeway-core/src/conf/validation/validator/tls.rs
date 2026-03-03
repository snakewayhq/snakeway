use crate::conf::validation::validator::read_nonempty_file;
use openssl::pkey::{PKey, Private};
use openssl::x509::X509;
use std::path::Path;

pub fn validate_cert_pem(path: &Path) -> Result<(), String> {
    let bytes = read_nonempty_file(path)?;

    X509::stack_from_pem(&bytes)
        .map_err(|e| format!("invalid PEM certificate {}: {e}", path.display()))?;

    Ok(())
}

pub fn validate_cert_key_pair(cert_path: &Path, key_path: &Path) -> Result<(), String> {
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
