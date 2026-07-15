//! Per-process temp storage for ACME test state.

use std::path::Path;
use std::sync::OnceLock;
use tempfile::TempDir;

/// Per-process root for ACME test state (order/account store and cert store).
///
/// Pebble, the acme test server, is ephemeral.
/// It forgets registered accounts when it restarts, and issued certs expire.
/// ACME state must therefore not persist across runs, or a stale account
/// key / expired cert poisons every subsequent run.
/// Each test process gets a unique temp directory, created once and shared by
/// the config builder and the runtime path patcher so both agree on the location.
/// nextest runs each test in its own process, so this yields fresh ACME state per test.
pub fn acme_test_root() -> &'static Path {
    static ROOT: OnceLock<TempDir> = OnceLock::new();
    ROOT.get_or_init(|| {
        let dir = TempDir::new().expect("failed to create ACME temp dir");
        std::fs::create_dir_all(dir.path().join("orders"))
            .expect("failed to create ACME orders directory");
        std::fs::create_dir_all(dir.path().join("certs"))
            .expect("failed to create ACME certs directory");
        dir
    })
    .path()
}
