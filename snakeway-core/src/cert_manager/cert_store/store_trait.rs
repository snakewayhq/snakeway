use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::SystemTime;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CertificateMeta {
    pub domains: Vec<String>,
    pub not_after: SystemTime,
    pub issued_at: SystemTime,
}

#[derive(Clone)]
pub struct StoredCertificate {
    pub private_key_pem: Vec<u8>,
    pub cert_chain_pem: Vec<u8>,
    pub meta: CertificateMeta,
}

pub trait CertStore: Send + Sync {
    fn get(&self, id: &str) -> Option<StoredCertificate>;

    fn put(&self, id: String, cert: StoredCertificate) -> Result<(), std::io::Error>;

    fn delete(&self, id: &str) -> Result<(), std::io::Error>;

    fn list(&self) -> Vec<(String, CertificateMeta)>;
}
