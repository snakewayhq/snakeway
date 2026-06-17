use crate::validation::validator::require_existing_file;
use confval::format::{
    Field, FieldKind, Fields, FromFields, Scalar, Value, ValueKind, parse_bool_field,
    parse_string_field, report_missing_field, report_unknown_field,
};
use confval::prelude::{Located, Report, Validate};
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

/// Lowers the opaque `config` blob from the neutral field model to an
/// `hcl::Value`, the structure the WASM module is handed untouched. Only
/// literal data survives the neutral model, so a non-literal expression (an
/// HCL template) is reported rather than silently dropped.
fn parse_value_field(field: &Field, report: &mut Report) -> Option<hcl::Value> {
    match &field.kind {
        FieldKind::Value(value) => value_to_hcl(value, report),
        FieldKind::Block(_) => {
            report
                .error("expected value, found block")
                .at(field.span)
                .emit();
            None
        }
    }
}

fn value_to_hcl(value: &Value, report: &mut Report) -> Option<hcl::Value> {
    match &value.kind {
        ValueKind::Scalar(Scalar::String(string)) => Some(hcl::Value::from(string.clone())),
        ValueKind::Scalar(Scalar::Int(int)) => Some(hcl::Value::from(*int)),
        ValueKind::Scalar(Scalar::Float(float)) => Some(hcl::Value::from(*float)),
        ValueKind::Scalar(Scalar::Bool(boolean)) => Some(hcl::Value::from(*boolean)),
        ValueKind::Seq(elements) => {
            let mut array = Vec::with_capacity(elements.len());
            for element in elements {
                array.push(value_to_hcl(element, report)?);
            }
            Some(hcl::Value::Array(array))
        }
        ValueKind::Map(fields) => {
            let mut object = hcl::Map::new();
            for field in fields.iter() {
                let FieldKind::Value(nested) = &field.kind else {
                    report
                        .error("expected value, found block")
                        .at(field.span)
                        .emit();
                    return None;
                };
                object.insert(field.name.clone(), value_to_hcl(nested, report)?);
            }
            Some(hcl::Value::Object(object))
        }
        ValueKind::Other(label) => {
            report
                .error(format!("unsupported value in config blob: {label}"))
                .at(value.span)
                .emit();
            None
        }
    }
}

impl FromFields for WasmDeviceSpec {
    fn from_fields(fields: &Fields, report: &mut Report) -> Option<Self> {
        let mut enable = None;
        let mut path = None;
        let mut config = None;

        for field in fields.iter() {
            match field.name.as_str() {
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

impl Validate for WasmDeviceSpec {
    fn validate(&self, report: &mut Report) {
        require_existing_file(&self.path, "wasm device", report);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use confval::format::hcl::parse_hcl;
    use confval::prelude::SourceMap;

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
