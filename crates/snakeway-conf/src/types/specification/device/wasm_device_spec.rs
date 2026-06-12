use confval::hcl::{
    Field, FieldKind, Fields, FromHcl, parse_bool_field, parse_string_field, report_missing_field,
    report_unknown_field,
};
use confval::provenance::{Located, Report};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Default, Debug, Clone, Serialize)]
pub struct WasmDeviceSpec {
    pub enable: Located<bool>,

    /// The location of the WASM module.
    pub path: Located<PathBuf>,

    /// Device-specific configuration blob, passed to the module untouched.
    pub config: Option<hcl::Value>,
}

fn parse_value_field(field: &Field<'_>, report: &mut Report) -> Option<hcl::Value> {
    match &field.kind {
        FieldKind::Value(expr) => {
            let expression: hcl::Expression = (*expr).clone().into();
            Some(expression.into())
        }
        FieldKind::Block(_) => {
            report
                .error("expected value, found block")
                .at(field.span)
                .emit();
            None
        }
    }
}

impl FromHcl for WasmDeviceSpec {
    fn from_hcl(fields: &Fields<'_>, report: &mut Report) -> Option<Self> {
        let mut enable = None;
        let mut path = None;
        let mut config = None;

        for field in fields.iter() {
            match field.name {
                "enable" => enable = parse_bool_field(field, report),
                "path" => {
                    path = parse_string_field(field, report).map(|value| value.map(PathBuf::from));
                }
                "config" => config = parse_value_field(field, report),
                _ => report_unknown_field(field, report),
            }
        }

        if enable.is_none() && !fields.has("enable") {
            report_missing_field("enable", fields.enclosing(), report);
        }
        if path.is_none() && !fields.has("path") {
            report_missing_field("path", fields.enclosing(), report);
        }

        Some(WasmDeviceSpec {
            enable: enable?,
            path: path?,
            config,
        })
    }
}

pub fn validate_wasm_device(spec: &WasmDeviceSpec, report: &mut Report) {
    if spec.path.value.as_os_str().is_empty() {
        report
            .error(format!(
                "wasm device path is empty: {}",
                spec.path.value.display()
            ))
            .at(spec.path.span)
            .emit();
    }
    if !spec.path.value.exists() {
        report
            .error(format!(
                "wasm device path does not exist: {}",
                spec.path.value.display()
            ))
            .at(spec.path.span)
            .emit();
    }
    if !spec.path.value.is_file() {
        report
            .error(format!(
                "wasm device path is not a file: {}",
                spec.path.value.display()
            ))
            .at(spec.path.span)
            .emit();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use confval::hcl::parse_hcl;
    use confval::provenance::SourceMap;

    #[test]
    fn parse_wasm_device_with_config_blob() {
        // Arrange
        let input =
            "enable = true\npath = \"./a.wasm\"\nconfig = { mode = \"strict\", retries = 3 }\n";
        let mut sources = SourceMap::new();
        let mut report = Report::new();
        let id = sources.add("device.hcl", input);

        // Act
        let spec = parse_hcl::<WasmDeviceSpec>(&sources, id, &mut report);

        // Assert
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
        let spec = spec.unwrap();
        assert!(spec.enable.value);
        assert_eq!(spec.path.value, PathBuf::from("./a.wasm"));
        let config = spec.config.unwrap();
        assert_eq!(
            config
                .as_object()
                .unwrap()
                .get("mode")
                .and_then(|v| v.as_str()),
            Some("strict")
        );
    }

    #[test]
    fn parse_wasm_device_without_config() {
        // Arrange
        let input = "enable = false\npath = \"./a.wasm\"\n";
        let mut sources = SourceMap::new();
        let mut report = Report::new();
        let id = sources.add("device.hcl", input);

        // Act
        let spec = parse_hcl::<WasmDeviceSpec>(&sources, id, &mut report);

        // Assert
        assert!(!report.has_issues());
        assert!(spec.unwrap().config.is_none());
    }
}
