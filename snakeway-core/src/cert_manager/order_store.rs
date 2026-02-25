use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderState {
    pub cert_id: String,

    pub status: OrderStatus,

    /// ACME order URL
    pub order_url: String,

    /// Authorization URLs returned by ACME
    pub authorization_urls: Vec<String>,

    /// Selected challenge auth URL (HTTP-01)
    pub challenge_url: Option<String>,

    /// HTTP-01 token
    pub challenge_token: Option<String>,

    /// keyAuthorization
    pub challenge_key_authorization: Option<String>,

    /// Failure count for backoff
    pub failure_count: u32,

    /// Last error string
    pub last_error: Option<String>,

    /// Last update timestamp
    pub updated_at: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OrderStatus {
    Ordering,
    ChallengeInit,
    Challenging,
    Finalizing,
    Failed,
}

pub trait OrderStore: Send + Sync {
    fn get(&self, cert_id: &str) -> std::io::Result<Option<OrderState>>;
    fn put(&self, state: &OrderState) -> std::io::Result<()>;
    fn delete(&self, cert_id: &str) -> std::io::Result<()>;
    fn list(&self) -> std::io::Result<Vec<OrderState>>;
}

pub struct FilesystemOrderStore {
    base_path: PathBuf,
}

impl FilesystemOrderStore {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    fn order_path(&self, cert_id: &str) -> PathBuf {
        self.base_path.join(cert_id).join("order.json")
    }
}

fn atomic_write(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    use std::fs;
    use std::io::Write;

    let tmp_path = path.with_extension("json.tmp");

    {
        let mut file = fs::File::create(&tmp_path)?;
        file.write_all(contents)?;
        file.sync_all()?;
    }

    fs::rename(tmp_path, path)?;

    Ok(())
}

impl OrderStore for FilesystemOrderStore {
    fn get(&self, cert_id: &str) -> std::io::Result<Option<OrderState>> {
        use std::fs;

        let path = self.order_path(cert_id);

        if !path.exists() {
            return Ok(None);
        }

        let data = fs::read(&path)?;
        let state = serde_json::from_slice(&data)?;

        Ok(Some(state))
    }
    fn put(&self, state: &OrderState) -> std::io::Result<()> {
        use std::fs;

        let path = self.order_path(&state.cert_id);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_vec_pretty(state)?;
        atomic_write(&path, &json)?;

        Ok(())
    }

    fn delete(&self, cert_id: &str) -> std::io::Result<()> {
        use std::fs;

        let path = self.order_path(cert_id);

        if path.exists() {
            fs::remove_file(path)?;
        }

        Ok(())
    }

    fn list(&self) -> std::io::Result<Vec<OrderState>> {
        use std::fs;

        let mut out = Vec::new();

        for entry in fs::read_dir(&self.base_path)? {
            let entry = entry?;
            let cert_id = entry.file_name().to_string_lossy().to_string();

            if let Some(state) = self.get(&cert_id)? {
                out.push(state);
            }
        }

        Ok(out)
    }
}
