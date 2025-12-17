use snakeway_core::logging::LogMode;
use anyhow::Result;
use serde_json::Value;
use std::io::{self, BufRead};

pub fn run_logs(mode: LogMode) -> Result<()> {
    let stdin = io::stdin();
    let reader = stdin.lock();

    for line in reader.lines() {
        let line = line?;

        // Fast path: raw passthrough
        if matches!(mode, LogMode::Raw) {
            println!("{line}");
            continue;
        }

        let Ok(json) = serde_json::from_str::<Value>(&line) else {
            // If it's not JSON, just print it.
            println!("{line}");
            continue;
        };

        render_event(&json, mode);
    }

    Ok(())
}

fn is_snakeway_event(event: &Value) -> bool {
    event.get("method").is_some() || event.get("uri").is_some() || event.get("status").is_some()
}

fn render_event(event: &Value, mode: LogMode) {
    let level = event.get("level").and_then(Value::as_str).unwrap_or("INFO");
    if is_snakeway_event(event) {
        render_snakeway_event(event, level, mode);
    } else {
        render_generic_event(event, level, mode);
    }
}

fn render_generic_event(event: &Value, level: &str, mode: LogMode) {
    let message = event
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("<no message>");

    let target = event.get("target").and_then(Value::as_str).unwrap_or("");

    match mode {
        LogMode::Pretty => {
            if target.is_empty() {
                println!("[{level}] {message}");
            } else {
                println!("[{level}] {message} ({target})");
            }
        }

        LogMode::Raw => unreachable!(),
    }
}

fn render_snakeway_event(event: &Value, level: &str, mode: LogMode) {
    let name = event
        .get("event")
        .and_then(Value::as_str)
        .unwrap_or("request");

    let method = event.get("method").and_then(Value::as_str);
    let uri = event.get("uri").and_then(Value::as_str);
    let status = event.get("status").and_then(Value::as_i64);

    match mode {
        LogMode::Pretty => {
            print!("[{level}] {name}");
            if let (Some(m), Some(u)) = (method, uri) {
                print!(" → {m} {u}");
            }
            if let Some(s) = status {
                print!(" ({s})");
            }
            println!();
        }

        LogMode::Raw => unreachable!(),
    }
}
