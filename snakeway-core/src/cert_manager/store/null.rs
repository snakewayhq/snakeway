use crate::cert_manager::CertStore;
use crate::cert_manager::store::{CertificateMeta, StoredCertificate};
use std::io::Error;

pub struct NullCertStore;

impl CertStore for NullCertStore {
    fn get(&self, _: &str) -> Option<StoredCertificate> {
        None
    }

    fn put(&self, _: String, _: StoredCertificate) -> Result<(), Error> {
        Err(Error::new(std::io::ErrorKind::Other, "cert store disabled"))
    }

    fn delete(&self, _: &str) -> Result<(), Error> {
        Ok(())
    }

    fn list(&self) -> Vec<(String, CertificateMeta)> {
        Vec::new()
    }
}
