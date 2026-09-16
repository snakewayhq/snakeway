use pingora_rustls::sign;
use snakeway_engine::runtime::{RuntimeState, TlsRuntime};
use std::collections::HashMap;
use std::sync::Arc;

pub(crate) fn make_test_certified_key() -> Arc<sign::CertifiedKey> {
    let cert = rcgen::generate_simple_self_signed(vec!["test.example".into()])
        .expect("failed to generate cert");

    let certs = snakeway_conf::tls::parse_cert_chain(cert.cert.pem().as_bytes())
        .expect("failed to parse cert PEM");
    let key = snakeway_conf::tls::parse_private_key(cert.signing_key.serialize_pem().as_bytes())
        .expect("failed to parse key PEM");

    Arc::new(
        snakeway_conf::tls::build_certified_key(certs, key).expect("failed to build CertifiedKey"),
    )
}

pub(crate) fn make_runtime_state(
    tls: Option<TlsRuntime>,
    manual_certs: HashMap<String, Arc<sign::CertifiedKey>>,
) -> RuntimeState {
    RuntimeState {
        tls,
        manual_certs,
        routers: HashMap::new(),
        devices: Default::default(),
        services: HashMap::new(),
    }
}
