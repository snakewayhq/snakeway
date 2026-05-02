use anyhow::{Context, Result};
use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;
use std::fs;
use std::path::Path;

/// Send SIGQUIT to a running Snakeway process via pid file.
///
/// This triggers Pingora's graceful upgrade path: the old process
/// serializes its listener FDs and sends them over the upgrade socket
/// to a new process started with `--upgrade`.
pub(crate) fn run<P: AsRef<Path>>(pid_file: P) -> Result<()> {
    let pid_file = pid_file.as_ref();

    let contents = fs::read_to_string(pid_file)
        .with_context(|| format!("failed to read pid file {}", pid_file.display()))?;

    let pid: i32 = contents
        .trim()
        .parse()
        .context("invalid pid file contents")?;

    let pid = Pid::from_raw(pid);

    kill(pid, Signal::SIGQUIT).with_context(|| format!("failed to send SIGQUIT to pid {}", pid))?;

    println!("Sent SIGQUIT to Snakeway (pid {})", pid);

    Ok(())
}
