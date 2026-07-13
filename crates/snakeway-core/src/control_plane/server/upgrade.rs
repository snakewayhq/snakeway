use std::path::Path;
use std::process::Command;
use tracing::{error, info};

/// Spawn a new Snakeway process in upgrade mode.
///
/// The new process will:
/// 1. Load and validate the config at `config_path`
/// 2. Send SIGQUIT to the old process (triggering FD transfer)
/// 3. Bootstrap with `--upgrade`, receiving FDs from the upgrade socket
/// 4. Begin serving on the inherited listener sockets
///
/// Returns `Ok(())` if the new process was spawned successfully. The old process
/// should continue serving until it receives SIGQUIT from the new process.
pub fn spawn_upgrade(config_path: &Path) -> anyhow::Result<()> {
    let exe = std::env::current_exe().map_err(|e| {
        anyhow::anyhow!("cannot determine current executable path for upgrade: {e}")
    })?;

    info!(
        binary = %exe.display(),
        config = %config_path.display(),
        "spawning upgrade process"
    );

    let child = Command::new(&exe)
        .arg("run")
        .arg("--config")
        .arg(config_path)
        .arg("--upgrade")
        .spawn();

    match child {
        Ok(child) => {
            info!(pid = child.id(), "upgrade process spawned");
            Ok(())
        }
        Err(e) => {
            error!(error = %e, "failed to spawn upgrade process");
            Err(anyhow::anyhow!("failed to spawn upgrade process: {e}"))
        }
    }
}
