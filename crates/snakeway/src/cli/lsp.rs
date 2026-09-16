use std::path::Path;

use confval::format::hcl::parse_hcl;
use confval::prelude::{Report, SourceMap};
use confval_lsp::{Hcl, LspError, Matcher, bind, serve_multi};
use glob::Pattern;
use snakeway_conf::types::{DevicesFile, EntrypointSpec, IncludeSpec, IngressSpec};

/// Which include list of the entrypoint a matcher routes against.
#[derive(Debug, Clone, Copy)]
enum IncludeKind {
    Devices,
    Ingresses,
}

/// Serves the config language server over stdio. One process serves all three
/// document shapes: the entrypoint by file name, and the included documents by
/// the include globs their own entrypoint declares.
pub(crate) fn run() -> Result<(), LspError> {
    serve_multi(vec![
        bind::<EntrypointSpec, Hcl>(Matcher::FileName("snakeway.hcl".to_string()), Hcl),
        bind::<DevicesFile, Hcl>(
            Matcher::Fn(Box::new(|path| {
                document_matches(path, IncludeKind::Devices)
            })),
            Hcl,
        ),
        bind::<IngressSpec, Hcl>(
            Matcher::Fn(Box::new(|path| {
                document_matches(path, IncludeKind::Ingresses)
            })),
            Hcl,
        ),
    ])
}

/// Whether a document belongs to the given include list.
///
/// The decision walks up from the document to the nearest directory holding a
/// `snakeway.hcl`, reads that entrypoint's include globs, and tests the
/// document's root-relative path against the matching glob, so the routing
/// follows the same patterns the loader discovers files with. Without a config
/// root, the default include globs are tested against the path's trailing
/// components, so a file in a conventional layout is served before its
/// entrypoint exists. This runs as a `Matcher::Fn` and must not panic, so
/// every failure answers no match.
fn document_matches(path: &Path, kind: IncludeKind) -> bool {
    match config_root(path) {
        Some(root) => {
            let glob = include_glob(&root.join("snakeway.hcl"), kind);
            let Ok(relative) = path.strip_prefix(root) else {
                return false;
            };
            Pattern::new(&glob).is_ok_and(|pattern| pattern.matches_path(relative))
        }
        None => trailing_components_match(path, &default_glob(kind)),
    }
}

/// The nearest ancestor directory that holds a `snakeway.hcl`, or `None`. The
/// document's own directory is searched first.
fn config_root(path: &Path) -> Option<&Path> {
    path.ancestors()
        .skip(1)
        .find(|dir| dir.join("snakeway.hcl").is_file())
}

/// The include glob the entrypoint declares for `kind`, or the default when
/// the entrypoint does not read. A failed read or parse answers the default
/// rather than an error, because the matcher's only vocabulary is match or no
/// match.
fn include_glob(entrypoint: &Path, kind: IncludeKind) -> String {
    let Ok(text) = std::fs::read_to_string(entrypoint) else {
        return default_glob(kind);
    };
    let mut sources = SourceMap::new();
    let id = sources.add(entrypoint.display().to_string(), text);
    let mut report = Report::new();
    let Some(entry) = parse_hcl::<EntrypointSpec>(&sources, id, &mut report) else {
        return default_glob(kind);
    };
    match kind {
        IncludeKind::Devices => entry.include.value.devices.value,
        IncludeKind::Ingresses => entry.include.value.ingresses.value,
    }
}

/// The include glob the parser fills when the entrypoint omits it.
fn default_glob(kind: IncludeKind) -> String {
    let defaults = IncludeSpec::default();
    match kind {
        IncludeKind::Devices => defaults.devices.value,
        IncludeKind::Ingresses => defaults.ingresses.value,
    }
}

/// Whether the trailing components of `path` match `glob`, comparing as many
/// components as the glob has. The default globs have a fixed shape, one
/// directory and one file pattern, so the comparison needs no config root.
fn trailing_components_match(path: &Path, glob: &str) -> bool {
    let Ok(pattern) = Pattern::new(glob) else {
        return false;
    };
    let depth = glob.split('/').count();
    let components: Vec<_> = path.components().collect();
    if components.len() < depth {
        return false;
    }
    let tail: std::path::PathBuf = components[components.len() - depth..].iter().collect();
    pattern.matches_path(&tail)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn write_entrypoint(root: &Path, devices: &str, ingresses: &str) {
        fs::write(
            root.join("snakeway.hcl"),
            format!(
                "server {{\n  version = 1\n}}\n\ninclude {{\n  devices = \"{devices}\"\n  ingresses = \"{ingresses}\"\n}}\n"
            ),
        )
        .unwrap();
    }

    #[test]
    fn a_device_document_matches_through_its_config_root() {
        // Arrange
        let dir = tempfile::tempdir().unwrap();
        write_entrypoint(dir.path(), "device.d/*.hcl", "ingress.d/*.hcl");
        let document = dir.path().join("device.d/identity.hcl");

        // Act, Assert
        assert!(document_matches(&document, IncludeKind::Devices));
        assert!(!document_matches(&document, IncludeKind::Ingresses));
    }

    #[test]
    fn a_customized_include_glob_reroutes_the_documents() {
        // Arrange
        let dir = tempfile::tempdir().unwrap();
        write_entrypoint(dir.path(), "my-devices/*.hcl", "ingress.d/*.hcl");

        // Act, Assert
        assert!(document_matches(
            &dir.path().join("my-devices/identity.hcl"),
            IncludeKind::Devices
        ));
        assert!(
            !document_matches(
                &dir.path().join("device.d/identity.hcl"),
                IncludeKind::Devices
            ),
            "the conventional directory no longer matches once the operator moved it"
        );
    }

    #[test]
    fn an_unreadable_entrypoint_falls_back_to_the_default_globs() {
        // Arrange
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("snakeway.hcl"), "not hcl {{{{").unwrap();

        // Act, Assert
        assert!(document_matches(
            &dir.path().join("device.d/identity.hcl"),
            IncludeKind::Devices
        ));
    }

    #[test]
    fn a_document_without_a_config_root_matches_the_conventional_layout() {
        // Arrange
        let document = PathBuf::from("/nonexistent/project/device.d/rate_limit.hcl");

        // Act, Assert
        assert!(document_matches(&document, IncludeKind::Devices));
        assert!(!document_matches(&document, IncludeKind::Ingresses));
        assert!(document_matches(
            &PathBuf::from("/nonexistent/project/ingress.d/api.hcl"),
            IncludeKind::Ingresses
        ));
    }

    #[test]
    fn an_unrelated_document_matches_nothing() {
        // Arrange
        let dir = tempfile::tempdir().unwrap();
        write_entrypoint(dir.path(), "device.d/*.hcl", "ingress.d/*.hcl");

        // Act, Assert
        assert!(!document_matches(
            &dir.path().join("notes/readme.hcl"),
            IncludeKind::Devices
        ));
        assert!(!document_matches(
            &dir.path().join("notes/readme.hcl"),
            IncludeKind::Ingresses
        ));
    }
}
