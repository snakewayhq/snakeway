use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// Write the current process PID to a file.
pub fn write_pid<P: AsRef<Path>>(path: P) -> Result<()> {
    let pid = std::process::id();
    fs::write(&path, pid.to_string())
        .with_context(|| format!("failed to write pid file {}", path.as_ref().display()))?;
    Ok(())
}

/// Remove a pid file (best-effort).
pub fn remove_pid<P: AsRef<Path>>(path: P) {
    let _ = fs::remove_file(path);
}
