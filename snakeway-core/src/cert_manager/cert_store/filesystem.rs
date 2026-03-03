use crate::cert_manager::CertStore;
use crate::cert_manager::cert_store::{CertificateMeta, StoredCertificate};

use std::io::Error;
use std::path::PathBuf;

pub struct FilesystemCertStore {
    path: PathBuf,
}

impl FilesystemCertStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
    fn cert_dir(&self, id: &str) -> PathBuf {
        self.path.join(id)
    }
}

impl CertStore for FilesystemCertStore {
    fn get(&self, id: &str) -> Option<StoredCertificate> {
        let dir = self.cert_dir(id);

        let key = std::fs::read(dir.join("key.pem")).ok()?;
        let cert = std::fs::read(dir.join("cert.pem")).ok()?;
        let meta_bytes = std::fs::read(dir.join("meta.json")).ok()?;

        let meta: CertificateMeta = serde_json::from_slice(&meta_bytes).ok()?;

        Some(StoredCertificate {
            private_key_pem: key,
            cert_chain_pem: cert,
            meta,
        })
    }

    fn put(&self, id: String, cert: StoredCertificate) -> Result<(), Error> {
        let final_dir = self.cert_dir(&id);
        let tmp_dir = self.cert_dir(&(id.clone() + ".tmp"));

        if tmp_dir.exists() {
            std::fs::remove_dir_all(&tmp_dir)?;
        }

        std::fs::create_dir_all(&tmp_dir)?;

        std::fs::write(tmp_dir.join("key.pem"), &*cert.private_key_pem)?;
        std::fs::write(tmp_dir.join("cert.pem"), &*cert.cert_chain_pem)?;

        let meta_json = serde_json::to_vec(&cert.meta).map_err(Error::other)?;

        std::fs::write(tmp_dir.join("meta.json"), meta_json)?;

        if final_dir.exists() {
            std::fs::remove_dir_all(&final_dir)?;
        }

        std::fs::rename(tmp_dir, final_dir)?;

        Ok(())
    }

    fn delete(&self, id: &str) -> Result<(), Error> {
        let dir = self.cert_dir(id);

        if dir.exists() {
            std::fs::remove_dir_all(dir)?;
        }

        Ok(())
    }

    fn list(&self) -> Vec<(String, CertificateMeta)> {
        let mut results = Vec::new();

        if let Ok(entries) = std::fs::read_dir(&self.path) {
            for entry in entries.flatten() {
                let id = match entry.file_name().into_string() {
                    Ok(s) => s,
                    Err(_) => continue,
                };

                let meta_path = entry.path().join("meta.json");

                if let Ok(meta_bytes) = std::fs::read(meta_path)
                    && let Ok(meta) = serde_json::from_slice::<CertificateMeta>(&meta_bytes)
                {
                    results.push((id, meta));
                }
            }
        }

        results
    }
}
