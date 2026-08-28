use crate::types::WasmDeviceFailPolicy;
use crate::validation::validator::require_existing_file;
use confval::prelude::{KeywordSet, Located, Report, Validate, range_constraint};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::PathBuf;

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

#[derive(Debug, Clone, Serialize, confval::Spec)]
pub struct WasmDeviceSpec {
    #[confval(label)]
    pub name: Located<String>,

    pub enable: Located<bool>,

    pub path: Located<PathBuf>,

    /// Behavior on guest trap, timeout, or load error: "open" or "closed".
    #[confval(keywords = WasmDeviceFailPolicy)]
    pub fail_policy: Located<String>,

    /// Per-hook epoch deadline in milliseconds.
    #[confval(default = 5, range = TIMEOUT_MS)]
    pub timeout_milliseconds: Located<i64>,

    /// Max body buffer bytes. 0 = streaming.
    #[confval(default = 0, range = BODY_BUFFER_MAX)]
    pub body_buffer_max_bytes: Located<i64>,

    /// Arbitrary key-value config passed to the guest via host.config-get.
    #[confval(map, default)]
    pub config: BTreeMap<String, Located<String>>,

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
            timeout_milliseconds: Located::detached(5),
            body_buffer_max_bytes: Located::detached(0),
            config: BTreeMap::new(),
            hooks: None,
        }
    }
}

impl Validate for WasmDeviceSpec {
    fn validate(&self, report: &mut Report) {
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

        if let Some(hooks) = &self.hooks {
            if hooks.value.is_empty() {
                report
                    .error("hooks must not be empty")
                    .at(hooks.span)
                    .help("omit `hooks` to run all hooks, or list at least one hook")
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
    use confval::format::FromFields;
    use confval::format::ToFields;
    use confval::format::hcl::{emit_hcl, parse_hcl};
    use confval::prelude::SourceMap;

    #[test]
    fn to_fields_round_trips_full_spec() {
        // Arrange
        let spec = WasmDeviceSpec {
            name: Located::detached("auth-filter".to_string()),
            enable: Located::detached(true),
            path: Located::detached(PathBuf::from("./a.wasm")),
            fail_policy: Located::detached("closed".to_string()),
            timeout_milliseconds: Located::detached(250),
            body_buffer_max_bytes: Located::detached(1024),
            config: BTreeMap::from([
                ("mode".to_string(), Located::detached("strict".to_string())),
                ("retries".to_string(), Located::detached("3".to_string())),
            ]),
            hooks: Some(Located::detached(vec![Located::detached(
                "on_request".to_string(),
            )])),
        };
        let mut report = Report::new();

        // Act
        let round_tripped = WasmDeviceSpec::from_fields(&spec.to_fields(), &mut report);

        // Assert
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
        let round_tripped = round_tripped.unwrap();
        assert_eq!(round_tripped.name.value, "auth-filter");
        assert!(round_tripped.enable.value);
        assert_eq!(round_tripped.path.value, PathBuf::from("./a.wasm"));
        assert_eq!(round_tripped.fail_policy.value, "closed");
        assert_eq!(round_tripped.timeout_milliseconds.value, 250);
        assert_eq!(round_tripped.body_buffer_max_bytes.value, 1024);
        assert_eq!(round_tripped.config.get("mode").unwrap().value, "strict");
        assert_eq!(round_tripped.config.get("retries").unwrap().value, "3");
        assert_eq!(round_tripped.hooks.unwrap().value[0].value, "on_request");
    }

    /// The populated walk always writes the map field, empty or not, while the
    /// source walk omits an empty one.
    #[test]
    fn empty_config_is_emitted_populated_and_omitted_from_source() {
        // Arrange
        let spec = WasmDeviceSpec {
            name: Located::detached("auth-filter".to_string()),
            enable: Located::detached(true),
            path: Located::detached(PathBuf::from("./a.wasm")),
            fail_policy: Located::detached("open".to_string()),
            ..WasmDeviceSpec::default()
        };

        // Act
        let populated = spec.to_fields();

        // Assert
        assert!(populated.has("config"));
        assert!(!populated.has("hooks"));
        assert!(!spec.to_source_fields().has("config"));
        assert!(populated.has("name"));
        assert!(populated.has("enable"));
        assert!(populated.has("path"));
        assert!(populated.has("fail_policy"));
        assert!(populated.has("timeout_milliseconds"));
        assert!(populated.has("body_buffer_max_bytes"));
    }

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
        assert_eq!(spec.timeout_milliseconds.value, 5);
        assert_eq!(spec.body_buffer_max_bytes.value, 0);
        assert_eq!(spec.config.get("mode").unwrap().value, "strict");
        assert_eq!(spec.config.get("retries").unwrap().value, "3");
    }

    #[test]
    fn parse_config_map_round_trips_with_sorted_keys() {
        // Arrange
        let input = r#"
name = "auth-filter"
enable = true
path = "./a.wasm"
fail_policy = "open"
config = { zulu = "z", alpha = "a", mike = "m" }
"#;
        let mut sources = SourceMap::new();
        let mut report = Report::new();
        let id = sources.add("device.hcl", input);

        // Act
        let spec = parse_hcl::<WasmDeviceSpec>(&sources, id, &mut report).unwrap();
        let emitted = emit_hcl(&spec.to_fields()).unwrap();

        // Assert
        let alpha = emitted.find("alpha").unwrap();
        let mike = emitted.find("mike").unwrap();
        let zulu = emitted.find("zulu").unwrap();
        assert!(alpha < mike && mike < zulu, "emitted: {emitted}");
        let mut emitted_sources = SourceMap::new();
        let emitted_id = emitted_sources.add("emitted.hcl", &emitted);
        let round_tripped = parse_hcl::<WasmDeviceSpec>(&emitted_sources, emitted_id, &mut report);
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
        assert_eq!(
            round_tripped.unwrap().config.get("zulu").unwrap().value,
            "z"
        );
    }

    #[test]
    fn duplicate_config_key_is_a_parse_error() {
        // Arrange
        let input = r#"
name = "auth-filter"
enable = true
path = "./a.wasm"
fail_policy = "open"
config {
  mode = "strict"
  mode = "lax"
}
"#;
        let mut sources = SourceMap::new();
        let mut report = Report::new();
        let id = sources.add("device.hcl", input);

        // Act
        let spec = parse_hcl::<WasmDeviceSpec>(&sources, id, &mut report);

        // Assert
        assert!(
            report.has_issues(),
            "a repeated config key must be reported, got {spec:?}"
        );
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
        let input = "name = \"plugin\"\nenable = true\npath = \"./a.wasm\"\nfail_policy = \"open\"\ntimeout_milliseconds = 100\nbody_buffer_max_bytes = 65536\n";
        let mut sources = SourceMap::new();
        let mut report = Report::new();
        let id = sources.add("device.hcl", input);

        // Act
        let spec = parse_hcl::<WasmDeviceSpec>(&sources, id, &mut report);

        // Assert
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
        let spec = spec.unwrap();
        assert_eq!(spec.timeout_milliseconds.value, 100);
        assert_eq!(spec.body_buffer_max_bytes.value, 65536);
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
        spec.validate_all(&mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("unknown fail_policy"))
        );
    }

    #[test]
    fn timeout_milliseconds_below_range_rejected() {
        // Arrange
        let mut report = Report::new();
        let spec = WasmDeviceSpec {
            name: Located::detached("test".to_string()),
            enable: Located::detached(true),
            path: Located::detached(PathBuf::from("/tmp/test.wasm")),
            fail_policy: Located::detached("open".to_string()),
            timeout_milliseconds: Located::detached(0),
            ..Default::default()
        };

        // Act
        spec.validate_all(&mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("timeout_milliseconds"))
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
    fn disabled_device_is_still_validated() {
        // Arrange
        let mut report = Report::new();
        let spec = WasmDeviceSpec {
            enable: Located::detached(false),
            fail_policy: Located::detached("sideways".to_string()),
            ..Default::default()
        };

        // Act
        spec.validate_all(&mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("fail_policy")),
            "a disabled device must still validate fail_policy; issues: {:?}",
            report.issues()
        );
    }
}
