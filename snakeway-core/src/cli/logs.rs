use crate::logging::LogMode;
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

        if let Some(event) = parse_event(&json) {
            handle_event(event, mode);
        }
    }

    Ok(())
}

//-----------------------------------------------------------------------------
// Event model
//-----------------------------------------------------------------------------

enum LogEvent {
    Snakeway(SnakewayEvent),
    Generic(GenericEvent),
}

struct SnakewayEvent {
    level: String,
    name: String,
    method: Option<String>,
    uri: Option<String>,
    status: Option<i64>,
}

struct GenericEvent {
    level: String,
    message: String,
    target: Option<String>,
}

//-----------------------------------------------------------------------------
// Parsing
//-----------------------------------------------------------------------------

fn is_snakeway_event(event: &Value) -> bool {
    event.get("method").is_some() || event.get("uri").is_some() || event.get("status").is_some()
}

fn parse_event(event: &Value) -> Option<LogEvent> {
    let level = event
        .get("level")
        .and_then(Value::as_str)
        .unwrap_or("INFO")
        .to_string();

    if is_snakeway_event(event) {
        Some(LogEvent::Snakeway(SnakewayEvent {
            level,
            name: event
                .get("event")
                .and_then(Value::as_str)
                .unwrap_or("request")
                .to_string(),
            method: event
                .get("method")
                .and_then(Value::as_str)
                .map(str::to_string),
            uri: event.get("uri").and_then(Value::as_str).map(str::to_string),
            status: event.get("status").and_then(Value::as_i64),
        }))
    } else {
        Some(LogEvent::Generic(GenericEvent {
            level,
            message: event
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("<no message>")
                .to_string(),
            target: event
                .get("target")
                .and_then(Value::as_str)
                .map(str::to_string),
        }))
    }
}

//-----------------------------------------------------------------------------
// Handling / Rendering
//-----------------------------------------------------------------------------

fn handle_event(event: LogEvent, mode: LogMode) {
    match mode {
        LogMode::Pretty => render_pretty(event),
        LogMode::Raw => unreachable!(),
    }
}

fn render_pretty(event: LogEvent) {
    match event {
        LogEvent::Snakeway(e) => {
            print!("[{}] {}", e.level, e.name);
            if let (Some(m), Some(u)) = (&e.method, &e.uri) {
                print!(" → {m} {u}");
            }
            if let Some(s) = e.status {
                print!(" ({s})");
            }
            println!();
        }

        LogEvent::Generic(e) => {
            if let Some(target) = e.target {
                println!("[{}] {} ({})", e.level, e.message, target);
            } else {
                println!("[{}] {}", e.level, e.message);
            }
        }
    }
}
