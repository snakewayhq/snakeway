use crate::cert_manager::CertStore;
use crate::cert_manager::store::{CertificateMeta, StoredCertificate};
use std::io::Error;
use std::path::PathBuf;

pub struct FilesystemCertStore {
    path: PathBuf,
}

impl FilesystemCertStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl CertStore for FilesystemCertStore {
    fn get(&self, id: &str) -> Option<StoredCertificate> {
        todo!()
    }

    fn put(&self, id: String, cert: StoredCertificate) -> Result<(), Error> {
        todo!()
    }

    fn delete(&self, id: &str) -> Result<(), Error> {
        todo!()
    }

    fn list(&self) -> Vec<(String, CertificateMeta)> {
        todo!()
    }
}
