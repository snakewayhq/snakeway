use pingora_rustls::sign;
use std::sync::Arc;

pub struct ParsedCert {
    certified_key: Arc<sign::CertifiedKey>,
}

impl ParsedCert {
    pub fn new(certified_key: Arc<sign::CertifiedKey>) -> Self {
        Self { certified_key }
    }

    pub fn certified_key(&self) -> &Arc<sign::CertifiedKey> {
        &self.certified_key
    }
}
