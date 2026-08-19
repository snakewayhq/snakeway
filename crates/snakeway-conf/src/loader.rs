use crate::discover::discover;
use crate::lower::lower_configs;
use crate::parse::flatten_devices;
use crate::types::RuntimeConfig;
use crate::types::{
    DeviceSpec, DevicesFile, EntrypointSpec, IngressSpec, ServerConfig, ServerSpec,
};
use crate::validation::{ConfigError, validate_spec};
use confval::format::FromFields;
use confval::format::hcl::{parse_hcl, parse_hcl_fields};
use confval::pipeline::check_references;
use confval::prelude::{Located, Lower, Report, SourceMap, Span};
use confval::schema::ToSchema;

use std::fs;
use std::path::Path;

#[derive(Debug)]
pub struct ValidatedConfig {
    pub config: RuntimeConfig,
    pub report: Report,
    pub sources: SourceMap,
}

impl ValidatedConfig {
    pub fn has_warnings(&self) -> bool {
        self.report.has_warnings()
    }

    pub fn render_plain(&self, out: &mut String) {
        self.report.render_plain(&self.sources, out).ok();
    }
}

#[hotpath::measure]
pub fn load_config(root: &Path) -> Result<ValidatedConfig, ConfigError> {
    let (sources, report, server_spec, device_specs, ingress_specs) = load_spec_files(root)?;
    load_config_from_parts(sources, report, server_spec, ingress_specs, device_specs)
}

/// Load configs from spec definitions.
/// Useful for integration testing where reading files is not necessarily scalable/maintainable.
pub fn load_config_from_specs(
    server_spec: ServerSpec,
    ingress_specs: Vec<IngressSpec>,
    device_specs: Vec<DeviceSpec>,
) -> Result<ValidatedConfig, ConfigError> {
    load_config_from_parts(
        SourceMap::new(),
        Report::new(),
        server_spec,
        ingress_specs.into_iter().map(Located::detached).collect(),
        device_specs.into_iter().map(Located::detached).collect(),
    )
}

fn load_config_from_parts(
    sources: SourceMap,
    mut report: Report,
    server_spec: ServerSpec,
    ingress_specs: Vec<Located<IngressSpec>>,
    device_specs: Vec<Located<DeviceSpec>>,
) -> Result<ValidatedConfig, ConfigError> {
    validate_spec(&server_spec, &ingress_specs, &device_specs, &mut report);

    // Lowering must not run on a report that contains errors.
    if report.has_errors() {
        return Err(ConfigError::SemanticValidationFailed { report, sources });
    }

    let server_config = ServerConfig::lower(&server_spec, &mut report);
    if report.has_errors() {
        return Err(ConfigError::SemanticValidationFailed { report, sources });
    }
    let server_config = server_config.ok_or(ConfigError::ServerLoweringReturnedNone)?;

    let config = lower_configs(server_config, ingress_specs, device_specs, &mut report);
    if report.has_errors() {
        return Err(ConfigError::SemanticValidationFailed { report, sources });
    }
    let config = config.ok_or(ConfigError::ConfigLoweringReturnedNone)?;

    Ok(ValidatedConfig {
        config,
        report,
        sources,
    })
}

pub(crate) type Spec = (
    SourceMap,
    Report,
    ServerSpec,
    Vec<Located<DeviceSpec>>,
    Vec<Located<IngressSpec>>,
);

/// Load spec from files
pub fn load_spec_files(root: &Path) -> Result<Spec, ConfigError> {
    let mut sources = SourceMap::new();
    let mut report = Report::new();

    //--------------------------------------------------------------------------
    // Entrypoint: span-first parsing
    //--------------------------------------------------------------------------
    let root_path = root.join("snakeway.hcl");
    let entry_text = fs::read_to_string(&root_path).map_err(|e| ConfigError::ReadFile {
        path: root.to_path_buf(),
        source: e,
    })?;
    let source_id = sources.add(root_path.display().to_string(), entry_text);

    // A parse that still produced a tree continues into validation so the
    // operator sees parse and validation problems in one pass; the error
    // gate before lowering stops the pipeline either way. Only a tree that
    // could not be built at all stops here.
    let entry: Option<EntrypointSpec> = parse_hcl(&sources, source_id, &mut report);
    let Some(entry) = entry else {
        debug_assert!(report.has_errors());
        return Err(ConfigError::SemanticValidationFailed { report, sources });
    };

    //--------------------------------------------------------------------------
    // Discover included files (hard fail)
    //--------------------------------------------------------------------------
    let device_files = discover(root, &entry.include.devices.value)?;
    let ingress_files = discover(root, &entry.include.ingresses.value)?;

    //--------------------------------------------------------------------------
    // Parse devices (span-first, same continue-on-Some semantics as ingress)
    //--------------------------------------------------------------------------
    let mut parsed_devices: Vec<Located<DeviceSpec>> = Vec::new();
    let mut any_unparseable = false;
    for path in &device_files {
        let text = fs::read_to_string(path).map_err(|e| ConfigError::ReadFile {
            path: path.clone(),
            source: e,
        })?;
        let source_id = sources.add(path.display().to_string(), &text);
        // The parse keeps the neutral field tree beside the typed spec, because
        // the reference pass resolves labels against the tree, not the spec.
        let fields = parse_hcl_fields(&sources, source_id, &mut report);
        let parsed = fields
            .as_ref()
            .and_then(|fields| DevicesFile::from_fields(fields, &mut report));
        if let Some(fields) = &fields {
            check_references(fields, &DevicesFile::schema(), &mut report);
        }
        match parsed {
            Some(file) => parsed_devices.extend(flatten_devices(file)),
            None => any_unparseable = true,
        }
    }

    //--------------------------------------------------------------------------
    // Parse ingress (span-first). A parse that still produced a tree
    // continues into validation; unparseable files stop the load after all
    // files have been read, so every syntax error is reported in one pass.
    //--------------------------------------------------------------------------
    let mut ingresses = Vec::new();
    for path in &ingress_files {
        let text = fs::read_to_string(path).map_err(|e| ConfigError::ReadFile {
            path: path.clone(),
            source: e,
        })?;
        let file_span = Span::new(
            sources.add(path.display().to_string(), &text),
            0,
            text.len() as u32,
        );
        let fields = parse_hcl_fields(&sources, file_span.source, &mut report);
        let parsed = fields
            .as_ref()
            .and_then(|fields| IngressSpec::from_fields(fields, &mut report));
        if let Some(fields) = &fields {
            check_references(fields, &IngressSpec::schema(), &mut report);
        }
        match parsed {
            Some(ingress) => ingresses.push(Located::new(ingress, file_span)),
            None => any_unparseable = true,
        }
    }
    if any_unparseable {
        return Err(ConfigError::SemanticValidationFailed { report, sources });
    }

    Ok((sources, report, entry.server, parsed_devices, ingresses))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write_config(dir: &Path, device_file: &str) {
        fs::write(
            dir.join("snakeway.hcl"),
            "server {\n  version = 1\n}\n\ninclude {\n  devices = \"device.d/*.hcl\"\n  ingresses = \"ingress.d/*.hcl\"\n}\n",
        )
        .unwrap();
        fs::create_dir_all(dir.join("device.d")).unwrap();
        fs::create_dir_all(dir.join("ingress.d")).unwrap();
        fs::write(dir.join("device.d/wasm.hcl"), device_file).unwrap();
    }

    #[test]
    fn duplicate_wasm_device_label_in_one_file_is_reported() {
        // Arrange
        let dir = tempfile::tempdir().unwrap();
        write_config(
            dir.path(),
            "wasm_devices \"auth\" {\n  enable = false\n  path = \"./a.wasm\"\n  fail_policy = \"open\"\n}\n\nwasm_devices \"auth\" {\n  enable = false\n  path = \"./b.wasm\"\n  fail_policy = \"open\"\n}\n",
        );

        // Act
        let (_, report, ..) = load_spec_files(dir.path()).unwrap();

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|i| i.message == "duplicate wasm_devices label \"auth\""),
            "issues: {:?}",
            report.issues()
        );
    }

    #[test]
    fn empty_wasm_device_label_is_reported_by_the_reference_pass() {
        // Arrange
        let dir = tempfile::tempdir().unwrap();
        write_config(
            dir.path(),
            "wasm_devices \"\" {\n  enable = false\n  path = \"./a.wasm\"\n  fail_policy = \"open\"\n}\n",
        );

        // Act
        let (_, report, ..) = load_spec_files(dir.path()).unwrap();

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|i| i.message == "a block label must not be empty"),
            "issues: {:?}",
            report.issues()
        );
    }
}
