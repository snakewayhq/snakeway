//! Process and signal helpers for tests that run the built binary.
//!
//! The spawn precedent comes from `tests/configuration/env_config.rs`: the
//! binary is built once into the workspace target dir and asserted to exist.
//! Signal sends go through nix, the same crate the binary itself uses.

use std::io::Read;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Once;
use std::time::{Duration, Instant};

static BUILD_ONCE: Once = Once::new();

/// The built snakeway binary, building it first when missing.
pub fn snakeway_bin() -> PathBuf {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let bin = workspace_root.join("target/debug/snakeway");

    BUILD_ONCE.call_once(|| {
        let status = Command::new("cargo")
            .args(["build", "-p", "snakeway"])
            .current_dir(&workspace_root)
            .status()
            .expect("failed to run cargo build");
        assert!(status.success(), "cargo build -p snakeway failed");
    });

    assert!(
        bin.exists(),
        "snakeway binary not found at {}",
        bin.display()
    );
    bin
}

/// A running snakeway proxy process started from a written config directory.
pub struct ProxyProcess {
    child: Child,
    pub pid: i32,
    /// The pid file path the config carries. Empty when the config has none.
    pub pid_file: PathBuf,
}

impl ProxyProcess {
    /// Starts the proxy against `config_dir` and waits until the listener
    /// accepts connections. `pid_file` must match the `server.pid_file` the
    /// config carries.
    pub fn start(config_dir: &Path, listener_port: u16, pid_file: PathBuf) -> Self {
        let bin = snakeway_bin();
        let child = Command::new(&bin)
            .args(["run", "--config"])
            .arg(config_dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("failed to spawn snakeway");
        let pid = child.id() as i32;

        let addr = format!("127.0.0.1:{listener_port}");
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            if TcpStream::connect(&addr).is_ok() {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "snakeway failed to start listening on {addr}"
            );
            std::thread::sleep(Duration::from_millis(50));
        }

        Self {
            child,
            pid,
            pid_file,
        }
    }

    /// Sends SIGTERM and waits for exit within `limit`. Returns false when
    /// the process outlives the limit.
    pub fn terminate_and_wait(&mut self, limit: Duration) -> bool {
        send_signal(self.pid, nix::sys::signal::Signal::SIGTERM);
        self.wait_within(limit)
    }

    /// Sends SIGHUP, which triggers a config reload.
    pub fn reload(&self) {
        send_signal(self.pid, nix::sys::signal::Signal::SIGHUP);
    }

    /// Sends SIGQUIT through the pid file the way the `upgrade` CLI command
    /// does, which starts the listener FD handoff.
    pub fn trigger_upgrade(&self) {
        let pid: i32 = std::fs::read_to_string(&self.pid_file)
            .expect("failed to read pid file")
            .trim()
            .parse()
            .expect("invalid pid file contents");
        send_signal(pid, nix::sys::signal::Signal::SIGQUIT);
    }

    /// Waits for the process to exit within `limit`.
    pub fn wait_within(&mut self, limit: Duration) -> bool {
        let deadline = Instant::now() + limit;
        loop {
            match self.child.try_wait().expect("failed to poll child") {
                Some(_) => return true,
                None => {
                    if Instant::now() >= deadline {
                        return false;
                    }
                    std::thread::sleep(Duration::from_millis(25));
                }
            }
        }
    }

    /// The exit code after the process ended, or None while it is still
    /// running.
    pub fn exit_code(&mut self) -> Option<i32> {
        self.child
            .try_wait()
            .expect("failed to poll child")
            .and_then(|status| status.code())
    }

    /// Starts a second proxy process in upgrade mode, which receives the
    /// listener FDs from the running instance.
    pub fn start_upgrade_process(config_dir: &Path, pid_file: PathBuf) -> Self {
        let bin = snakeway_bin();
        let child = Command::new(&bin)
            .args(["run", "--config"])
            .arg(config_dir)
            .arg("--upgrade")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("failed to spawn upgrade process");
        Self {
            pid: child.id() as i32,
            child,
            pid_file,
        }
    }
}

impl Drop for ProxyProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_file(&self.pid_file);
    }
}

/// Sends a signal to a process id. Test-only helper around nix::kill.
pub fn send_signal(pid: i32, signal: nix::sys::signal::Signal) {
    nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid), signal).expect("failed to send signal");
}

/// Waits until a TCP connection to `addr` succeeds or the deadline passes.
/// Returns false on timeout.
pub fn wait_for_connect(addr: &str, limit: Duration) -> bool {
    let deadline = Instant::now() + limit;
    loop {
        if TcpStream::connect(addr).is_ok() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

/// Waits until a TCP connection to `addr` fails or is ended without service,
/// which is how a refused new connection shows up after shutdown. A connect
/// error, an immediate EOF, or a reset counts as refused. A read timeout
/// means the peer accepted and is still holding the connection, which is not
/// a refusal. Returns false when the deadline passed without a refusal.
pub fn wait_for_refused(addr: &str, limit: Duration) -> bool {
    let deadline = Instant::now() + limit;
    loop {
        match TcpStream::connect(addr) {
            Err(_) => return true,
            Ok(mut stream) => {
                let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
                let mut buf = [0u8; 1];
                match stream.read(&mut buf) {
                    Ok(0) => return true,
                    Err(e)
                        if e.kind() == std::io::ErrorKind::WouldBlock
                            || e.kind() == std::io::ErrorKind::TimedOut =>
                    {
                        // The peer accepted and holds the connection.
                    }
                    Err(_) => return true,
                    Ok(_) => {}
                }
            }
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}
