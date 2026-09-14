use crate::runtime::types::ManualCertMap;
use anyhow::{Result, anyhow};
use pingora::tls::sign::CertifiedKey;
use snakeway_conf::tls::{CertKeyError, build_certified_key, parse_cert_chain, parse_private_key};
use snakeway_conf::types::{ListenerConfig, TlsTerminationConfig};
use std::path::Path;
use std::sync::Arc;

/// Load the certificate and private key of every listener that uses manual TLS.
///
/// The files are read each time the runtime state is built. A reload therefore serves
/// files that were replaced at the same paths, and a reload that fails to load them
/// leaves the previous runtime state and its certificates in place.
pub(crate) fn load_manual_certs(listeners: &[ListenerConfig]) -> Result<ManualCertMap> {
    let mut certs = ManualCertMap::new();

    for listener in listeners {
        let Some(TlsTerminationConfig::Manual { cert, key }) = &listener.tls_termination else {
            continue;
        };

        let certified_key = load_certified_key(cert, key)
            .map_err(|e| anyhow!("manual TLS for listener {}: {e}", listener.name))?;
        certs.insert(listener.addr.clone(), Arc::new(certified_key));
    }

    Ok(certs)
}

fn load_certified_key(cert_path: &Path, key_path: &Path) -> Result<CertifiedKey> {
    let cert_bytes = std::fs::read(cert_path).map_err(|e| {
        anyhow!(
            "failed to read certificate file {}: {e}",
            cert_path.display()
        )
    })?;
    let key_bytes = std::fs::read(key_path).map_err(|e| {
        anyhow!(
            "failed to read private key file {}: {e}",
            key_path.display()
        )
    })?;

    let certs =
        parse_cert_chain(&cert_bytes).map_err(|e| anyhow!("{}: {e}", cert_path.display()))?;
    let key = parse_private_key(&key_bytes).map_err(|e| anyhow!("{}: {e}", key_path.display()))?;

    build_certified_key(certs, key).map_err(|e| match e {
        CertKeyError::KeyMismatch => anyhow!(
            "private key does not match certificate: cert={}, key={}",
            cert_path.display(),
            key_path.display()
        ),
        CertKeyError::Other(msg) => anyhow!(
            "failed to build certified key: cert={}, key={}: {msg}",
            cert_path.display(),
            key_path.display()
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn listener(
        name: &str,
        addr: &str,
        tls_termination: Option<TlsTerminationConfig>,
    ) -> ListenerConfig {
        ListenerConfig {
            name: name.to_string(),
            addr: addr.to_string(),
            tls_termination,
            enable_http2: false,
            http2: None,
            enable_admin: false,
            admin_auth: None,
            redirect: None,
            connection_filter: None,
            connection_rate_limiting_filter: None,
        }
    }

    fn write_pair(dir: &Path, name: &str) -> (PathBuf, PathBuf, Vec<u8>) {
        let generated = rcgen::generate_simple_self_signed(vec!["localhost".into()])
            .expect("failed to generate self-signed cert");
        let cert_path = dir.join(format!("{name}.pem"));
        let key_path = dir.join(format!("{name}.key"));
        std::fs::write(&cert_path, generated.cert.pem()).expect("failed to write cert");
        std::fs::write(&key_path, generated.signing_key.serialize_pem())
            .expect("failed to write key");
        (cert_path, key_path, generated.cert.der().to_vec())
    }

    #[test]
    fn load_manual_certs_keys_certificate_by_bind_address() {
        // Arrange
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let (cert, key, der) = write_pair(dir.path(), "traffic");
        let listeners = vec![listener(
            "traffic",
            "127.0.0.1:8443",
            Some(TlsTerminationConfig::Manual { cert, key }),
        )];

        // Act
        let result = load_manual_certs(&listeners);

        // Assert
        let certs = result.expect("manual certificates must load");
        assert_eq!(certs.len(), 1);
        let certified = certs
            .get("127.0.0.1:8443")
            .expect("certificate must be keyed by the bind address");
        assert_eq!(certified.cert.len(), 1);
        assert_eq!(certified.cert[0].as_ref(), der.as_slice());
    }

    #[test]
    fn load_manual_certs_skips_listeners_without_manual_tls() {
        // Arrange
        let listeners = vec![listener("plain", "127.0.0.1:8080", None)];

        // Act
        let result = load_manual_certs(&listeners);

        // Assert
        assert!(
            result
                .expect("listeners without TLS must not fail")
                .is_empty()
        );
    }

    #[test]
    fn load_manual_certs_rejects_mismatched_key() {
        // Arrange
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let (cert, _, _) = write_pair(dir.path(), "first");
        let (_, key, _) = write_pair(dir.path(), "second");
        let listeners = vec![listener(
            "traffic",
            "127.0.0.1:8443",
            Some(TlsTerminationConfig::Manual {
                cert: cert.clone(),
                key: key.clone(),
            }),
        )];

        // Act
        let result = load_manual_certs(&listeners);

        // Assert
        let msg = result.expect_err("a mismatched key must fail").to_string();
        assert!(
            msg.starts_with("manual TLS for listener traffic: "),
            "got: {msg}"
        );
        assert!(
            msg.contains("private key does not match certificate"),
            "got: {msg}"
        );
        assert!(msg.contains(&cert.display().to_string()), "got: {msg}");
        assert!(msg.contains(&key.display().to_string()), "got: {msg}");
    }

    #[test]
    fn load_manual_certs_reports_missing_certificate_file() {
        // Arrange
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let (_, key, _) = write_pair(dir.path(), "traffic");
        let missing = dir.path().join("missing.pem");
        let listeners = vec![listener(
            "traffic",
            "127.0.0.1:8443",
            Some(TlsTerminationConfig::Manual {
                cert: missing.clone(),
                key,
            }),
        )];

        // Act
        let result = load_manual_certs(&listeners);

        // Assert
        let msg = result.expect_err("a missing file must fail").to_string();
        assert!(
            msg.starts_with("manual TLS for listener traffic: "),
            "got: {msg}"
        );
        assert!(
            msg.contains(&format!(
                "failed to read certificate file {}",
                missing.display()
            )),
            "got: {msg}"
        );
    }
}
