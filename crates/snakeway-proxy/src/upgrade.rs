use std::path::Path;

/// Send SIGQUIT to the old process to trigger FD transfer.
///
/// Called by the NEW process (started with `--upgrade`) right before
/// `server.bootstrap()` blocks on the upgrade socket. The old process's
/// `send_fds_to` retries on ENOENT/ECONNREFUSED, so it is safe to send
/// SIGQUIT before the new process has created the upgrade socket.
#[cfg(unix)]
pub(crate) fn signal_old_process(pid_file: &Path) -> anyhow::Result<()> {
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

    tracing::info!(
        old_pid = pid,
        "sending SIGQUIT to old process for FD transfer"
    );
    kill(Pid::from_raw(pid), Signal::SIGQUIT)
        .map_err(|e| anyhow::anyhow!("failed to send SIGQUIT to pid {pid}: {e}"))?;

    Ok(())
}

#[cfg(not(unix))]
pub(crate) fn signal_old_process(_pid_file: &Path) -> anyhow::Result<()> {
    tracing::warn!("zero-drop upgrade is only supported on Linux; skipping SIGQUIT");
    Ok(())
}
