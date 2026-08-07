use crate::cli::logs::types::{GenericEvent, IdentitySummary, LogEvent, SnakewayEvent};
use serde_json::Value;
use std::time::SystemTime;

fn is_snakeway_event(event: &Value) -> bool {
    event.get("method").is_some() || event.get("uri").is_some() || event.get("status").is_some()
}

pub(crate) fn parse_event(event: &Value) -> Option<LogEvent> {
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

                // All log values are strings, (e.g., "true", "false")
                let bot = parsed
                    .get("bot")
                    .and_then(Value::as_str)
                    .and_then(|b| b.parse::<bool>().ok());

                let asn = parsed
                    .get("asn")
                    .and_then(Value::as_str)
                    .and_then(|s| s.parse::<usize>().ok());
                let aso = parsed.get("aso").and_then(Value::as_str).map(String::from);
                let connection_type = parsed
                    .get("connection_type")
                    .and_then(Value::as_str)
                    .map(String::from);
                let country = parsed
                    .get("country")
                    .and_then(Value::as_str)
                    .map(String::from);

                if device.is_some() || bot.is_some() {
                    Some(IdentitySummary {
                        device,
                        bot,
                        asn,
                        aso,
                        connection_type,
                        country,
                    })
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
            // All log values are strings, (e.g., "200")
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

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use serde_json::json;

    #[test]
    fn should_parse_snakeway_event_fields() {
        // Arrange
        let raw = json!({
            "level": "WARN",
            "event": "response",
            "request_id": "abc123",
            "method": "GET",
            "uri": "/api/users",
            "status": "503",
            "timestamp": "2026-01-01T00:00:00Z",
        });

        // Act
        let event = parse_event(&raw);

        // Assert
        let Some(LogEvent::Snakeway(e)) = event else {
            panic!("expected a snakeway event");
        };
        assert_eq!(e.level, "WARN");
        assert_eq!(e.name, "response");
        assert_eq!(e.request_id.as_deref(), Some("abc123"));
        assert_eq!(e.method.as_deref(), Some("GET"));
        assert_eq!(e.uri.as_deref(), Some("/api/users"));
        assert_eq!(e.status, Some(503));
        assert!(e.ts.is_some(), "rfc3339 timestamp must parse");
    }

    #[test]
    fn should_default_event_name_and_level() {
        // Arrange
        let raw = json!({ "method": "GET" });

        // Act
        let event = parse_event(&raw);

        // Assert
        let Some(LogEvent::Snakeway(e)) = event else {
            panic!("expected a snakeway event");
        };
        assert_eq!(e.name, "request");
        assert_eq!(e.level, "INFO");
        assert_eq!(e.status, None);
    }

    #[test]
    fn should_parse_identity_from_encoded_json() {
        // Arrange
        let raw = json!({
            "method": "GET",
            "identity": r#"{"device":"phone","bot":"true","asn":"64512","aso":"TestNet","connection_type":"cellular","country":"NZ"}"#,
        });

        // Act
        let event = parse_event(&raw);

        // Assert
        let Some(LogEvent::Snakeway(e)) = event else {
            panic!("expected a snakeway event");
        };
        let identity = e.identity.expect("identity must parse");
        assert_eq!(identity.device.as_deref(), Some("phone"));
        assert_eq!(identity.bot, Some(true));
        assert_eq!(identity.asn, Some(64512));
        assert_eq!(identity.aso.as_deref(), Some("TestNet"));
        assert_eq!(identity.connection_type.as_deref(), Some("cellular"));
        assert_eq!(identity.country.as_deref(), Some("NZ"));
    }

    #[test]
    fn should_drop_identity_without_device_or_bot() {
        // Arrange
        let raw = json!({
            "method": "GET",
            "identity": r#"{"country":"NZ"}"#,
        });

        // Act
        let event = parse_event(&raw);

        // Assert
        let Some(LogEvent::Snakeway(e)) = event else {
            panic!("expected a snakeway event");
        };
        assert!(e.identity.is_none());
    }

    #[test]
    fn should_parse_generic_event_with_target() {
        // Arrange
        let raw = json!({
            "level": "DEBUG",
            "message": "reload complete",
            "target": "snakeway::server",
        });

        // Act
        let event = parse_event(&raw);

        // Assert
        let Some(LogEvent::Generic(e)) = event else {
            panic!("expected a generic event");
        };
        assert_eq!(e.level, "DEBUG");
        assert_eq!(e.message, "reload complete");
        assert_eq!(e.target.as_deref(), Some("snakeway::server"));
    }

    #[test]
    fn should_fall_back_to_a_placeholder_message() {
        // Arrange
        let raw = json!({ "level": "INFO" });

        // Act
        let event = parse_event(&raw);

        // Assert
        let Some(LogEvent::Generic(e)) = event else {
            panic!("expected a generic event");
        };
        assert_eq!(e.message, "<no message>");
        assert_eq!(e.target, None);
    }
}
