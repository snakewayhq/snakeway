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

/// Send SIGQUIT to the old process to trigger FD transfer.
///
/// Called by the NEW process (started with `--upgrade`) right before
/// `server.bootstrap()` blocks on the upgrade socket. The old process's
/// `send_fds_to` retries on ENOENT/ECONNREFUSED, so it is safe to send
/// SIGQUIT before the new process has created the upgrade socket.
#[cfg(unix)]
pub fn signal_old_process(pid_file: &Path) -> anyhow::Result<()> {
    use nix::sys::signal::{Signal, kill};
    use nix::unistd::Pid;

    if pid_file.as_os_str().is_empty() {
        anyhow::bail!(
            "pid_file is not configured; cannot send SIGQUIT to old process. \
             Set server.pid_file in config to enable zero-drop upgrades."
        );
    }

    let pid_str = std::fs::read_to_string(pid_file)
        .map_err(|e| anyhow::anyhow!("failed to read pid file {}: {e}", pid_file.display()))?;

    let pid: i32 = pid_str.trim().parse().map_err(|e| {
        anyhow::anyhow!(
            "invalid pid in {}: '{}' ({e})",
            pid_file.display(),
            pid_str.trim()
        )
    })?;

    info!(
        old_pid = pid,
        "sending SIGQUIT to old process for FD transfer"
    );
    kill(Pid::from_raw(pid), Signal::SIGQUIT)
        .map_err(|e| anyhow::anyhow!("failed to send SIGQUIT to pid {pid}: {e}"))?;

    Ok(())
}

#[cfg(not(unix))]
pub fn signal_old_process(_pid_file: &Path) -> anyhow::Result<()> {
    tracing::warn!("zero-drop upgrade is only supported on Linux; skipping SIGQUIT");
    Ok(())
}
