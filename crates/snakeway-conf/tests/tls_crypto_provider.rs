use snakeway_conf::tls::{build_certified_key, parse_cert_chain, parse_private_key};

#[test]
fn build_certified_key_installs_crypto_provider_when_none_is_installed() {
    // Arrange
    assert!(
        pingora_rustls::CryptoProvider::get_default().is_none(),
        "this test binary must start without a process-level crypto provider"
    );
    let generated = rcgen::generate_simple_self_signed(vec!["localhost".into()])
        .expect("failed to generate self-signed cert");
    let certs = parse_cert_chain(generated.cert.pem().as_bytes()).expect("failed to parse cert");
    let key = parse_private_key(generated.signing_key.serialize_pem().as_bytes())
        .expect("failed to parse key");

    // Act
    let result = build_certified_key(certs, key);

    // Assert
    let certified = result.expect("build_certified_key must not require a preinstalled provider");
    assert_eq!(certified.cert.len(), 1);
    assert_eq!(certified.cert[0].as_ref(), generated.cert.der().as_ref());
    assert!(pingora_rustls::CryptoProvider::get_default().is_some());
}
