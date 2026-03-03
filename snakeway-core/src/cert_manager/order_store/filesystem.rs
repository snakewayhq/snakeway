use crate::cert_manager::order_store::store_trait::{OrderState, OrderStore};
use std::path::{Path, PathBuf};

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
