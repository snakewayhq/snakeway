use anyhow::{Context, Result};
use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;
use std::fs;
use std::path::Path;

/// Send SIGHUP to a running Snakeway process via pid file.
pub fn run<P: AsRef<Path>>(pid_file: P) -> Result<()> {
    let pid_file = pid_file.as_ref();

    let contents = fs::read_to_string(pid_file)
        .with_context(|| format!("failed to read pid file {}", pid_file.display()))?;

    let pid: i32 = contents
        .trim()
        .parse()
        .context("invalid pid file contents")?;

    let pid = Pid::from_raw(pid);

    // Send SIGHUP
    kill(pid, Signal::SIGHUP).with_context(|| format!("failed to send SIGHUP to pid {}", pid))?;

    println!("Sent SIGHUP to Snakeway (pid {})", pid);

    Ok(())
}
