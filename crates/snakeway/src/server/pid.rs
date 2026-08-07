use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// Write the current process PID to a file.
pub(crate) fn write_pid<P: AsRef<Path>>(path: P) -> Result<()> {
    let pid = std::process::id();
    fs::write(&path, pid.to_string())
        .with_context(|| format!("failed to write pid file {}", path.as_ref().display()))?;
    Ok(())
}

/// Remove a pid file (best-effort).
pub(crate) fn remove_pid<P: AsRef<Path>>(path: P) {
    let _ = fs::remove_file(path);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_write_current_process_id() {
        // Arrange
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("snakeway.pid");

        // Act
        let result = write_pid(&path);

        // Assert
        assert!(result.is_ok());
        let content = fs::read_to_string(&path).expect("pid file must exist");
        assert_eq!(content, std::process::id().to_string());
    }

    #[test]
    fn should_error_when_pid_directory_is_missing() {
        // Arrange
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("missing").join("snakeway.pid");

        // Act
        let result = write_pid(&path);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn should_remove_pid_file() {
        // Arrange
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("snakeway.pid");
        write_pid(&path).expect("write");

        // Act
        remove_pid(&path);

        // Assert
        assert!(!path.exists());
    }

    #[test]
    fn should_ignore_removing_a_missing_pid_file() {
        // Arrange
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("absent.pid");

        // Act
        remove_pid(&path);

        // Assert
        assert!(!path.exists());
    }
}
