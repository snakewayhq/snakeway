use std::fs;
use std::path::Path;

pub(crate) fn read_nonempty_file(path: &Path) -> Result<Vec<u8>, String> {
    if !path.is_file() {
        return Err(format!("file does not exist: {}", path.display()));
    }

    let bytes = fs::read(path).map_err(|e| format!("cannot read file {}: {e}", path.display()))?;

    if bytes.is_empty() {
        return Err(format!("file is empty: {}", path.display()));
    }

    Ok(bytes)
}
