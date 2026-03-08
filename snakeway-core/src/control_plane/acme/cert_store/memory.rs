use crate::acme::cert_store::{CertStore, CertificateMeta, StoredCertificate};
use std::collections::HashMap;
use std::sync::RwLock;

pub struct MemoryCertStore {
    inner: RwLock<HashMap<String, StoredCertificate>>,
}

impl Default for MemoryCertStore {
    fn default() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }
}

impl CertStore for MemoryCertStore {
    fn get(&self, id: &str) -> Option<StoredCertificate> {
        self.inner.read().unwrap().get(id).cloned()
    }

    fn put(&self, id: String, cert: StoredCertificate) -> Result<(), std::io::Error> {
        self.inner.write().unwrap().insert(id, cert);
        Ok(())
    }

    fn delete(&self, id: &str) -> Result<(), std::io::Error> {
        self.inner.write().unwrap().remove(id);
        Ok(())
    }

    fn list(&self) -> Vec<(String, CertificateMeta)> {
        self.inner
            .read()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.meta.clone()))
            .collect()
    }
}
