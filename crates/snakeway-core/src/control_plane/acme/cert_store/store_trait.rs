use secrecy::{ExposeSecret, SecretBox};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CertificateMeta {
    pub(crate) domains: Vec<String>,
    pub(crate) not_after: SystemTime,
    pub(crate) issued_at: SystemTime,
}

pub struct StoredCertificate {
    pub(crate) private_key_pem: SecretBox<Vec<u8>>,
    pub(crate) cert_chain_pem: Vec<u8>,
    pub(crate) meta: CertificateMeta,
}

impl StoredCertificate {
    pub(crate) fn new(
        private_key_pem: Vec<u8>,
        cert_chain_pem: Vec<u8>,
        meta: CertificateMeta,
    ) -> Self {
        Self {
            private_key_pem: SecretBox::new(Box::new(private_key_pem)),
            cert_chain_pem,
            meta,
        }
    }

    pub(crate) fn expose_private_key_pem(&self) -> &[u8] {
        self.private_key_pem.expose_secret()
    }
}

impl Clone for StoredCertificate {
    fn clone(&self) -> Self {
        Self {
            private_key_pem: SecretBox::new(Box::new(self.private_key_pem.expose_secret().clone())),
            cert_chain_pem: self.cert_chain_pem.clone(),
            meta: self.meta.clone(),
        }
    }
}

pub trait CertStore: Send + Sync {
    fn get(&self, id: &str) -> Option<StoredCertificate>;

    fn put(&self, id: String, cert: StoredCertificate) -> Result<(), std::io::Error>;

    fn delete(&self, id: &str) -> Result<(), std::io::Error>;

    fn list(&self) -> Vec<(String, CertificateMeta)>;
}
