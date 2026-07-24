use crate::types::HclInt;
use crate::validation::validator::require_existing_file;
use confval::format::{
    Field, FieldKind, Fields, FromFields, Scalar, ValueKind, parse_bool_field, parse_int_field,
    parse_string_field, parse_string_list_field, report_missing_field, report_unknown_field,
};
use confval::prelude::{KeywordSet, Located, Report, Validate, ValidateNested};
use confval::{RangeConstraint, range_constraint};
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;

pub const FAIL_POLICIES: [&str; 2] = ["open", "closed"];

/// Lifecycle hooks a WASM device may declare via the `hooks` allowlist.
pub const HOOK_NAMES: [&str; 6] = [
    "on_request",
    "on_stream_request_body",
    "before_proxy",
    "after_proxy",
    "on_stream_response_body",
    "on_response",
];

range_constraint!(TIMEOUT_MS, i64, min: 1, max: 60000);
range_constraint!(BODY_BUFFER_MAX, i64, min: 0, max: 104857600);

#[derive(Debug, Clone, Serialize)]
pub struct WasmDeviceSpec {
    pub name: Located<String>,

    pub enable: Located<bool>,

    pub path: Located<PathBuf>,

    /// Behavior on guest trap, timeout, or load error: "open" or "closed".
    pub fail_policy: Located<String>,

    /// Per-hook epoch deadline in milliseconds.
    pub timeout_ms: Located<HclInt>,

    /// Max body buffer bytes. 0 = streaming.
    pub body_buffer_max: Located<HclInt>,

    /// Arbitrary key-value config passed to the guest via host.config-get.
    pub config: HashMap<String, String>,

    /// Lifecycle hooks this device implements.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hooks: Option<Located<Vec<Located<String>>>>,
}

impl Default for WasmDeviceSpec {
    fn default() -> Self {
        Self {
            name: Located::detached(String::new()),
            enable: Located::detached(false),
            path: Located::detached(PathBuf::new()),
            fail_policy: Located::detached("open".to_string()),
            timeout_ms: Located::detached(5),
            body_buffer_max: Located::detached(0),
            config: HashMap::new(),
            hooks: None,
        }
    }
}

/// Parses the `config` attribute as a flat string-to-string map.
fn parse_config_map(field: &Field, report: &mut Report) -> Option<HashMap<String, String>> {
    let value = match &field.kind {
        FieldKind::Value(value) => value,
        FieldKind::Block(_) => {
            report
                .error("expected map value for config, found block")
                .at(field.span)
                .emit();
            return None;
        }
    };
    match &value.kind {
        ValueKind::Map(fields) => {
            let mut map = HashMap::new();
            for f in fields.iter() {
                let FieldKind::Value(nested) = &f.kind else {
                    report
                        .error("expected value in config map, found block")
                        .at(f.span)
                        .emit();
                    return None;
                };
                match &nested.kind {
                    ValueKind::Scalar(Scalar::String(s)) => {
                        map.insert(f.name.clone(), s.clone());
                    }
                    _ => {
                        report
                            .error(format!(
                                "config values must be strings, found non-string for key \"{}\"",
                                f.name
                            ))
                            .at(nested.span)
                            .emit();
                        return None;
                    }
                }
            }
            Some(map)
        }
        _ => {
            report
                .error("expected map for config")
                .at(value.span)
                .emit();
            None
        }
    }
}

impl FromFields for WasmDeviceSpec {
    fn from_fields(fields: &Fields, report: &mut Report) -> Option<Self> {
        let mut name = None;
        let mut enable = None;
        let mut path = None;
        let mut fail_policy = None;
        let mut timeout_ms = None;
        let mut body_buffer_max = None;
        let mut config = HashMap::new();
        let mut hooks = None;

        for field in fields.iter() {
            match field.name.as_str() {
                "name" => name = parse_string_field(field, report),
                "enable" => enable = parse_bool_field(field, report),
                "path" => {
                    path = parse_string_field(field, report).map(|value| value.map(PathBuf::from));
                }
                "fail_policy" => fail_policy = parse_string_field(field, report),
                "timeout_ms" => timeout_ms = parse_int_field(field, report),
                "body_buffer_max" => body_buffer_max = parse_int_field(field, report),
                "config" => {
                    if let Some(parsed) = parse_config_map(field, report) {
                        config = parsed;
                    }
                }
                "hooks" => hooks = parse_string_list_field(field, report),
                _ => report_unknown_field(field, report),
            }
        }

        if name.is_none() && !fields.has("name") {
            report_missing_field("name", fields.enclosing(), report);
        }
        if enable.is_none() && !fields.has("enable") {
            report_missing_field("enable", fields.enclosing(), report);
        }
        if path.is_none() && !fields.has("path") {
            report_missing_field("path", fields.enclosing(), report);
        }
        if fail_policy.is_none() && !fields.has("fail_policy") {
            report_missing_field("fail_policy", fields.enclosing(), report);
        }

        Some(WasmDeviceSpec {
            name: name?,
            enable: enable?,
            path: path?,
            fail_policy: fail_policy?,
            timeout_ms: timeout_ms.unwrap_or(Located::detached(5)),
            body_buffer_max: body_buffer_max.unwrap_or(Located::detached(0)),
            config,
            hooks,
        })
    }
}

impl ValidateNested for WasmDeviceSpec {
    fn validate_nested(&self, _report: &mut Report) {}
}

impl Validate for WasmDeviceSpec {
    fn validate(&self, report: &mut Report) {
        if !self.enable.value {
            return;
        }

        if self.name.value.trim().is_empty() {
            report
                .error("wasm device name must not be empty")
                .at(self.name.span)
                .emit();
        }

        require_existing_file(
            &self.path,
            "wasm device",
            Some("Provide a path to a compiled .wasm module."),
            report,
        );

        KeywordSet::new(&FAIL_POLICIES).check_located(&self.fail_policy, "fail_policy", report);

        TIMEOUT_MS.check_located(&self.timeout_ms, "timeout_ms", report);
        BODY_BUFFER_MAX.check_located(&self.body_buffer_max, "body_buffer_max", report);

        if let Some(hooks) = &self.hooks {
            if hooks.value.is_empty() {
                report
                    .error("hooks must not be empty")
                    .at(hooks.span)
                    .help("omit `hooks` to run all hooks, or set enable = false to disable the device")
                    .emit();
            }
            for hook in &hooks.value {
                KeywordSet::new(&HOOK_NAMES).check_located(hook, "hook", report);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use confval::format::hcl::parse_hcl;
    use confval::prelude::SourceMap;

    #[test]
    fn parse_wasm_device_with_config_map() {
        // Arrange
        let input = r#"
name = "auth-filter"
enable = true
path = "./a.wasm"
fail_policy = "open"
config = { mode = "strict", retries = "3" }
"#;
        let mut sources = SourceMap::new();
        let mut report = Report::new();
        let id = sources.add("device.hcl", input);

        // Act
        let spec = parse_hcl::<WasmDeviceSpec>(&sources, id, &mut report);

        // Assert
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
        let spec = spec.unwrap();
        assert_eq!(spec.name.value, "auth-filter");
        assert!(spec.enable.value);
        assert_eq!(spec.path.value, PathBuf::from("./a.wasm"));
        assert_eq!(spec.fail_policy.value, "open");
        assert_eq!(spec.timeout_ms.value, 5);
        assert_eq!(spec.body_buffer_max.value, 0);
        assert_eq!(spec.config.get("mode").unwrap(), "strict");
        assert_eq!(spec.config.get("retries").unwrap(), "3");
    }

    #[test]
    fn parse_wasm_device_without_config() {
        // Arrange
        let input =
            "name = \"plugin\"\nenable = false\npath = \"./a.wasm\"\nfail_policy = \"closed\"\n";
        let mut sources = SourceMap::new();
        let mut report = Report::new();
        let id = sources.add("device.hcl", input);

        // Act
        let spec = parse_hcl::<WasmDeviceSpec>(&sources, id, &mut report);

        // Assert
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
        let spec = spec.unwrap();
        assert!(spec.config.is_empty());
        assert_eq!(spec.fail_policy.value, "closed");
    }

    #[test]
    fn parse_wasm_device_with_explicit_timeout() {
        // Arrange
        let input = "name = \"plugin\"\nenable = true\npath = \"./a.wasm\"\nfail_policy = \"open\"\ntimeout_ms = 100\nbody_buffer_max = 65536\n";
        let mut sources = SourceMap::new();
        let mut report = Report::new();
        let id = sources.add("device.hcl", input);

        // Act
        let spec = parse_hcl::<WasmDeviceSpec>(&sources, id, &mut report);

        // Assert
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
        let spec = spec.unwrap();
        assert_eq!(spec.timeout_ms.value, 100);
        assert_eq!(spec.body_buffer_max.value, 65536);
    }

    #[test]
    fn empty_name_rejected() {
        // Arrange
        let mut report = Report::new();
        let spec = WasmDeviceSpec {
            name: Located::detached("  ".to_string()),
            enable: Located::detached(true),
            path: Located::detached(PathBuf::from("/tmp/test.wasm")),
            fail_policy: Located::detached("open".to_string()),
            ..Default::default()
        };

        // Act
        spec.validate(&mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("name must not be empty"))
        );
    }

    #[test]
    fn unknown_fail_policy_rejected() {
        // Arrange
        let mut report = Report::new();
        let spec = WasmDeviceSpec {
            name: Located::detached("test".to_string()),
            enable: Located::detached(true),
            path: Located::detached(PathBuf::from("/tmp/test.wasm")),
            fail_policy: Located::detached("maybe".to_string()),
            ..Default::default()
        };

        // Act
        spec.validate(&mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("unknown fail_policy"))
        );
    }

    #[test]
    fn timeout_ms_below_range_rejected() {
        // Arrange
        let mut report = Report::new();
        let spec = WasmDeviceSpec {
            name: Located::detached("test".to_string()),
            enable: Located::detached(true),
            path: Located::detached(PathBuf::from("/tmp/test.wasm")),
            fail_policy: Located::detached("open".to_string()),
            timeout_ms: Located::detached(0),
            ..Default::default()
        };

        // Act
        spec.validate(&mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("timeout_ms"))
        );
    }

    // -- Hooks allowlist --

    #[test]
    fn parse_wasm_device_with_hooks() {
        // Arrange
        let input = r#"
name = "auth"
enable = true
path = "./a.wasm"
fail_policy = "closed"
hooks = ["on_request", "on_response"]
"#;
        let mut sources = SourceMap::new();
        let mut report = Report::new();
        let id = sources.add("device.hcl", input);

        // Act
        let spec = parse_hcl::<WasmDeviceSpec>(&sources, id, &mut report);

        // Assert
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
        let spec = spec.unwrap();
        let hooks = spec.hooks.expect("hooks present");
        let names: Vec<&str> = hooks.value.iter().map(|h| h.value.as_str()).collect();
        assert_eq!(names, vec!["on_request", "on_response"]);
    }

    #[test]
    fn unknown_hook_rejected() {
        // Arrange
        let mut report = Report::new();
        let spec = WasmDeviceSpec {
            name: Located::detached("test".to_string()),
            enable: Located::detached(true),
            path: Located::detached(PathBuf::from("/tmp/test.wasm")),
            fail_policy: Located::detached("open".to_string()),
            hooks: Some(Located::detached(vec![Located::detached(
                "on_bogus".to_string(),
            )])),
            ..Default::default()
        };

        // Act
        spec.validate(&mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("unknown hook: on_bogus"))
        );
    }

    #[test]
    fn empty_hooks_rejected() {
        // Arrange
        let mut report = Report::new();
        let spec = WasmDeviceSpec {
            name: Located::detached("test".to_string()),
            enable: Located::detached(true),
            path: Located::detached(PathBuf::from("/tmp/test.wasm")),
            fail_policy: Located::detached("open".to_string()),
            hooks: Some(Located::detached(vec![])),
            ..Default::default()
        };

        // Act
        spec.validate(&mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("hooks must not be empty"))
        );
    }

    #[test]
    fn disabled_device_skips_validation() {
        // Arrange
        let mut report = Report::new();
        let spec = WasmDeviceSpec {
            enable: Located::detached(false),
            fail_policy: Located::detached("sideways".to_string()),
            ..Default::default()
        };

        // Act
        spec.validate(&mut report);

        // Assert
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
    }
}
