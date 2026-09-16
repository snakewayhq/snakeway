use crate::runtime::types::UpstreamTlsRuntime;
use anyhow::{Context, Result, anyhow};
use pingora::utils::tls::WrappedX509;
use snakeway_conf::types::UpstreamTlsConfig;
use std::fs;
use std::path::Path;
use std::sync::Arc;

/// Resolve the TLS settings of one upstream.
///
/// An upstream without a `tls` block has no TLS settings. The per-upstream `ca_file` is used
/// before the global `server.ca_file`. Without either, the peer carries no CA and Pingora
/// verifies against the trust store of its connector.
pub(crate) fn resolve_upstream_tls(
    tls: Option<&UpstreamTlsConfig>,
    sni: String,
    global_ca_file: Option<&Path>,
) -> Result<Option<UpstreamTlsRuntime>> {
    let Some(tls_cfg) = tls else {
        return Ok(None);
    };
    let mut resolved = UpstreamTlsRuntime {
        sni,
        verify: tls_cfg.verify,
        ca: None,
        group_key: 0,
    };
    if tls_cfg.verify
        && let Some(ca_file) = tls_cfg.ca_file.as_deref().or(global_ca_file)
    {
        resolved.ca = Some(Arc::from(load_ca_from_path(ca_file)?));
        resolved.group_key = calculate_group_key(ca_file);
    }
    Ok(Some(resolved))
}

/// Load a per-upstream CA file.
/// This happens when the runtime state is recomputed,
/// keeping it out of the data plane.
pub(crate) fn load_ca_from_path(path: &Path) -> Result<Vec<WrappedX509>> {
    if !path.exists() {
        anyhow::bail!("CA file does not exist: {}", path.display());
    }
    if !path.is_file() {
        anyhow::bail!("CA path is not a file: {}", path.display());
    }

    let pem =
        fs::read(path).with_context(|| format!("failed to read CA file: {}", path.display()))?;
    if pem.is_empty() {
        anyhow::bail!("CA file is empty: {}", path.display());
    }

    let parsed = snakeway_conf::tls::parse_ca_certificates(&pem)
        .map_err(|e| anyhow!("CA file {}: {e}", path.display()))?;

    parsed
        .into_iter()
        .map(|cert_der| wrap_ca_certificate(cert_der.to_vec()))
        .collect()
}

/// The closure must return Pingora's `X509Certificate` type, so this only compiles while
/// Snakeway and Pingora resolve the same x509-parser version. If a dependency update splits
/// them, align the versions again rather than working around the type error.
fn wrap_ca_certificate(der: Vec<u8>) -> Result<WrappedX509> {
    WrappedX509::try_new(der, |raw| {
        x509_parser::parse_x509_certificate(raw)
            .map(|(_, cert)| cert)
            .map_err(|e| anyhow!("invalid X.509 certificate: {e}"))
    })
}

/// Hash a path to a u64.
/// This is used to group per-upstream CAs,
/// keeping them out of the data plane.
fn calculate_group_key(path: &Path) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = ahash::AHasher::default();
    path.as_os_str().hash(&mut h);
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tls_config(verify: bool, ca_file: Option<&Path>) -> UpstreamTlsConfig {
        UpstreamTlsConfig {
            sni: "backend.test".to_string(),
            verify,
            ca_file: ca_file.map(Path::to_path_buf),
        }
    }

    fn write_ca_file(dir: &Path, name: &str) -> (std::path::PathBuf, Vec<u8>) {
        let cert = rcgen::generate_simple_self_signed(vec![format!("{name}.test")])
            .expect("failed to generate CA cert");
        let path = dir.join(format!("{name}.pem"));
        std::fs::write(&path, cert.cert.pem()).expect("failed to write CA file");
        (path, cert.cert.der().to_vec())
    }

    #[test]
    fn upstream_without_tls_block_has_no_tls_settings() {
        // Arrange
        let tls = None;

        // Act
        let result = resolve_upstream_tls(tls, String::new(), None);

        // Assert
        assert!(result.expect("resolution must succeed").is_none());
    }

    #[test]
    fn verify_false_skips_the_ca_file() {
        // Arrange
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let (ca_file, _) = write_ca_file(dir.path(), "upstream");
        let cfg = tls_config(false, Some(&ca_file));

        // Act
        let result = resolve_upstream_tls(Some(&cfg), "backend.test".to_string(), None);

        // Assert
        let tls = result
            .expect("resolution must succeed")
            .expect("a tls block must resolve");
        assert!(!tls.verify);
        assert!(tls.ca.is_none());
        assert_eq!(tls.group_key, 0);
    }

    #[test]
    fn verify_without_any_ca_file_keeps_verification_on() {
        // Arrange
        let cfg = tls_config(true, None);

        // Act
        let result = resolve_upstream_tls(Some(&cfg), "backend.test".to_string(), None);

        // Assert
        let tls = result
            .expect("resolution must succeed")
            .expect("a tls block must resolve");
        assert_eq!(tls.sni, "backend.test");
        assert!(tls.verify);
        assert!(tls.ca.is_none());
        assert_eq!(tls.group_key, 0);
    }

    #[test]
    fn global_ca_file_applies_when_upstream_has_none() {
        // Arrange
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let (global, global_der) = write_ca_file(dir.path(), "global");
        let cfg = tls_config(true, None);

        // Act
        let result = resolve_upstream_tls(Some(&cfg), "backend.test".to_string(), Some(&global));

        // Assert
        let tls = result
            .expect("resolution must succeed")
            .expect("a tls block must resolve");
        let ca = tls.ca.expect("the global CA must apply");
        assert_eq!(ca.len(), 1);
        assert_eq!(ca[0].borrow_raw_cert().as_slice(), global_der.as_slice());
        assert_eq!(tls.group_key, calculate_group_key(&global));
    }

    #[test]
    fn upstream_ca_file_is_used_before_global_ca_file() {
        // Arrange
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let (global, _) = write_ca_file(dir.path(), "global");
        let (upstream, upstream_der) = write_ca_file(dir.path(), "upstream");
        let cfg = tls_config(true, Some(&upstream));

        // Act
        let result = resolve_upstream_tls(Some(&cfg), "backend.test".to_string(), Some(&global));

        // Assert
        let tls = result
            .expect("resolution must succeed")
            .expect("a tls block must resolve");
        let ca = tls.ca.expect("the upstream CA must apply");
        assert_eq!(ca.len(), 1);
        assert_eq!(ca[0].borrow_raw_cert().as_slice(), upstream_der.as_slice());
        assert_eq!(tls.group_key, calculate_group_key(&upstream));
    }

    #[test]
    fn load_ca_from_path_valid_single_cert() {
        // Arrange
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let ca_path = dir.path().join("ca.pem");
        let cert = rcgen::generate_simple_self_signed(vec!["ca.test".into()])
            .expect("failed to generate CA cert");
        std::fs::write(&ca_path, cert.cert.pem()).expect("failed to write CA");

        // Act
        let result = load_ca_from_path(&ca_path);

        // Assert
        let certs = result.expect("a single CA certificate must load");
        assert_eq!(certs.len(), 1);
        assert_eq!(
            certs[0].borrow_raw_cert().as_slice(),
            cert.cert.der().as_ref()
        );
    }

    #[test]
    fn load_ca_from_path_valid_multi_cert_bundle() {
        // Arrange
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let ca_path = dir.path().join("ca-bundle.pem");
        let cert1 = rcgen::generate_simple_self_signed(vec!["ca1.test".into()])
            .expect("failed to generate first CA cert");
        let cert2 = rcgen::generate_simple_self_signed(vec!["ca2.test".into()])
            .expect("failed to generate second CA cert");
        let mut f = std::fs::File::create(&ca_path).expect("failed to create file");
        f.write_all(cert1.cert.pem().as_bytes())
            .expect("failed to write first cert");
        f.write_all(cert2.cert.pem().as_bytes())
            .expect("failed to write second cert");

        // Act
        let result = load_ca_from_path(&ca_path);

        // Assert
        let certs = result.expect("a CA bundle must load");
        assert_eq!(certs.len(), 2);
        assert_eq!(
            certs[0].borrow_raw_cert().as_slice(),
            cert1.cert.der().as_ref()
        );
        assert_eq!(
            certs[1].borrow_raw_cert().as_slice(),
            cert2.cert.der().as_ref()
        );
    }

    #[test]
    fn load_ca_from_path_nonexistent_file() {
        // Arrange
        let path = Path::new("/nonexistent/ca.pem");

        // Act
        let result = load_ca_from_path(path);

        // Assert
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("does not exist"), "got: {msg}");
    }

    #[test]
    fn load_ca_from_path_empty_file() {
        // Arrange
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let ca_path = dir.path().join("empty.pem");
        std::fs::File::create(&ca_path).expect("failed to create empty file");

        // Act
        let result = load_ca_from_path(&ca_path);

        // Assert
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("empty"), "got: {msg}");
    }

    #[test]
    fn load_ca_from_path_no_pem_certs() {
        // Arrange
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let ca_path = dir.path().join("no-certs.pem");
        std::fs::write(&ca_path, "not a PEM file").expect("failed to write");

        // Act
        let result = load_ca_from_path(&ca_path);

        // Assert
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("no certificates"), "got: {msg}");
    }

    #[test]
    fn load_ca_from_path_invalid_der_in_pem() {
        // Arrange
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let ca_path = dir.path().join("bad-der.pem");
        std::fs::write(
            &ca_path,
            "-----BEGIN CERTIFICATE-----\naW52YWxpZA==\n-----END CERTIFICATE-----\n",
        )
        .expect("failed to write");

        // Act
        let result = load_ca_from_path(&ca_path);

        // Assert
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("invalid X.509 certificate at index 0"),
            "got: {msg}"
        );
    }

    #[test]
    fn load_ca_from_path_rejects_private_key_section() {
        // Arrange
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let ca_path = dir.path().join("ca-with-key.pem");
        let cert = rcgen::generate_simple_self_signed(vec!["ca.test".into()])
            .expect("failed to generate CA cert");
        std::fs::write(
            &ca_path,
            format!("{}{}", cert.cert.pem(), cert.signing_key.serialize_pem()),
        )
        .expect("failed to write CA file");

        // Act
        let result = load_ca_from_path(&ca_path);

        // Assert
        let msg = result
            .expect_err("a CA file with a private key section must fail to load")
            .to_string();
        assert!(msg.contains("PRIVATE KEY"), "got: {msg}");
    }

    #[test]
    fn wrap_ca_certificate_invalid_der_returns_error() {
        // Arrange
        let der = b"invalid".to_vec();

        // Act
        let result = wrap_ca_certificate(der);

        // Assert
        let msg = result
            .expect_err("invalid DER must return an error")
            .to_string();
        assert!(msg.starts_with("invalid X.509 certificate"), "got: {msg}");
    }
}
