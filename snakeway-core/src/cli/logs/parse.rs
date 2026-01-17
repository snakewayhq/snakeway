use crate::cli::logs::types::{GenericEvent, IdentitySummary, LogEvent, SnakewayEvent};
use serde_json::Value;
use std::time::SystemTime;

fn is_snakeway_event(event: &Value) -> bool {
    event.get("method").is_some() || event.get("uri").is_some() || event.get("status").is_some()
}

pub fn parse_event(event: &Value) -> Option<LogEvent> {
    let level = event
        .get("level")
        .and_then(Value::as_str)
        .unwrap_or("INFO")
        .to_string();

    if is_snakeway_event(event) {
        Some(LogEvent::Snakeway(SnakewayEvent {
            level,
            request_id: event
                .get("request_id")
                .and_then(Value::as_str)
                .map(str::to_string),
            ts: event
                .get("timestamp")
                .and_then(Value::as_str)
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(SystemTime::from),
            identity: event.get("identity").and_then(|v| {
                // identity is a JSON-encoded string
                let s = v.as_str()?;
                let Ok(parsed) = serde_json::from_str::<Value>(s) else {
                    return None;
                };

                let device = parsed
                    .get("device")
                    .and_then(Value::as_str)
                    .map(String::from);

                // In your logs bot is a string ("true"/"false")
                let bot = parsed
                    .get("bot")
                    .and_then(Value::as_str)
                    .and_then(|b| b.parse::<bool>().ok());

                if device.is_some() || bot.is_some() {
                    Some(IdentitySummary { device, bot })
                } else {
                    None
                }
            }),
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
            // status is a string in your logs (e.g. "200")
            status: event
                .get("status")
                .and_then(Value::as_str)
                .and_then(|s| s.parse::<i64>().ok()),
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
