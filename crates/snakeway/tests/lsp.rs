#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Test helpers fail tests by panicking. The clippy.toml test carve-out does not reach them."
)]

use std::io::{Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

fn frame(payload: &serde_json::Value) -> Vec<u8> {
    let body = payload.to_string();
    format!("Content-Length: {}\r\n\r\n{body}", body.len()).into_bytes()
}

/// Every framed message on the reader, read on a helper thread until the
/// stream closes. The server stays alive after a shutdown until its client
/// closes it, so the caller kills the child and then drains the channel.
fn spawn_reader(child: &mut Child) -> mpsc::Receiver<serde_json::Value> {
    let mut stdout = child.stdout.take().expect("child stdout");
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut data = Vec::new();
        let mut buffer = [0u8; 4096];
        while let Ok(count) = stdout.read(&mut buffer) {
            if count == 0 {
                break;
            }
            data.extend_from_slice(&buffer[..count]);
            while let Some((message, rest)) = next_frame(&data) {
                let _ = sender.send(message);
                data = rest;
            }
        }
    });
    receiver
}

fn next_frame(data: &[u8]) -> Option<(serde_json::Value, Vec<u8>)> {
    let header_end = data.windows(4).position(|w| w == b"\r\n\r\n")?;
    let header = std::str::from_utf8(&data[..header_end]).ok()?;
    let length: usize = header
        .lines()
        .find_map(|line| {
            line.to_ascii_lowercase()
                .strip_prefix("content-length:")
                .map(str::trim)
                .map(String::from)
        })
        .and_then(|value| value.parse().ok())?;
    let body_start = header_end + 4;
    if data.len() < body_start + length {
        return None;
    }
    let message = serde_json::from_slice(&data[body_start..body_start + length]).ok()?;
    Some((message, data[body_start + length..].to_vec()))
}

/// The editor and `snakeway config check` run the same pipeline, so the same
/// broken device file must report the same message in both. The fixture is the
/// one `config_check.rs` uses, with the same out of range value.
#[test]
fn lsp_diagnostics_match_config_check_for_a_device_document() {
    // Arrange
    let mut child = Command::new(env!("CARGO_BIN_EXE_snakeway"))
        .arg("lsp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn snakeway lsp");
    let receiver = spawn_reader(&mut child);
    let mut stdin = child.stdin.take().expect("child stdin");
    let device_text = "request_rate_limiting_device {\n  enable = false\n  max_requests_per_second = 0\n  window_seconds = 5\n  paths = []\n}\n";

    // Act
    stdin
        .write_all(&frame(&serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": { "capabilities": {} }
        })))
        .unwrap();
    stdin
        .write_all(&frame(&serde_json::json!({
            "jsonrpc": "2.0", "method": "initialized", "params": {}
        })))
        .unwrap();
    stdin
        .write_all(&frame(&serde_json::json!({
            "jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": { "textDocument": {
                "uri": "file:///cfg/device.d/rate_limit.hcl",
                "languageId": "hcl", "version": 1, "text": device_text
            }}
        })))
        .unwrap();
    stdin.flush().unwrap();

    let mut diagnostics = None;
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    while std::time::Instant::now() < deadline {
        let Ok(message) = receiver.recv_timeout(Duration::from_secs(5)) else {
            break;
        };
        if message["method"] == "textDocument/publishDiagnostics" {
            diagnostics = Some(message["params"]["diagnostics"].clone());
            break;
        }
    }
    child.kill().unwrap();
    child.wait().unwrap();

    // Assert
    let diagnostics = diagnostics.expect("the server publishes diagnostics for the device file");
    let messages: Vec<&str> = diagnostics
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["message"].as_str().unwrap())
        .collect();
    assert_eq!(
        messages,
        ["max_requests_per_second must be at least 1"],
        "the editor must report exactly what `snakeway config check` reports for this file"
    );
}
