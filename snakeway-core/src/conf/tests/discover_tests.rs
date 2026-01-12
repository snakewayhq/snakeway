use crate::conf::discover::{discover, resolve_glob};
use crate::conf::validation::ConfigError;

use std::fs;
use std::path::Path;
use tempfile::tempdir;

#[test]
fn discover_finds_matching_files() {
    // Arrange
    let dir = tempdir().unwrap();
    let root = dir.path();

    fs::write(root.join("a.hcl"), "").unwrap();
    fs::write(root.join("b.hcl"), "").unwrap();
    fs::write(root.join("c.txt"), "").unwrap();

    // Act
    let result = discover(root, "*.hcl").unwrap();

    // Assert
    assert_eq!(result, vec![root.join("a.hcl"), root.join("b.hcl"),]);
}

#[test]
fn discover_returns_sorted_paths() {
    // Arrange
    let dir = tempdir().unwrap();
    let root = dir.path();

    fs::write(root.join("z.hcl"), "").unwrap();
    fs::write(root.join("a.hcl"), "").unwrap();
    fs::write(root.join("m.hcl"), "").unwrap();

    // Act
    let result = discover(root, "*.hcl").unwrap();

    // Assert
    assert_eq!(
        result,
        vec![root.join("a.hcl"), root.join("m.hcl"), root.join("z.hcl"),]
    );
}

#[test]
fn discover_supports_recursive_globs() {
    // Arrange
    let dir = tempdir().unwrap();
    let root = dir.path();

    fs::create_dir_all(root.join("nested/inner")).unwrap();
    fs::write(root.join("root.hcl"), "").unwrap();
    fs::write(root.join("nested/a.hcl"), "").unwrap();
    fs::write(root.join("nested/inner/b.hcl"), "").unwrap();

    // Act
    let result = discover(root, "**/*.hcl").unwrap();

    // Assert
    assert_eq!(
        result,
        vec![
            root.join("nested/a.hcl"),
            root.join("nested/inner/b.hcl"),
            root.join("root.hcl"),
        ]
    );
}

#[test]
fn discover_returns_empty_vec_when_no_matches() {
    // Arrange
    let dir = tempdir().unwrap();
    let root = dir.path();

    fs::write(root.join("a.txt"), "").unwrap();

    // Act
    let result = discover(root, "*.hcl").unwrap();

    // Assert
    assert!(result.is_empty());
}

#[test]
fn discover_filters_out_directories() {
    // Arrange
    let dir = tempdir().unwrap();
    let root = dir.path();

    fs::create_dir(root.join("config.hcl")).unwrap();

    // Act
    let result = discover(root, "*.hcl").unwrap();

    // Assert
    assert!(result.is_empty());
}

#[test]
fn discover_returns_error_for_invalid_glob() {
    // Arrange
    let dir = tempdir().unwrap();
    let root = dir.path();

    // Act
    let err = discover(root, "[").unwrap_err();

    // Assert
    match err {
        ConfigError::Glob { pattern, .. } => {
            assert!(pattern.contains('['));
        }
        other => panic!("unexpected error: {:?}", other),
    }
}

#[test]
fn resolve_glob_joins_root_and_pattern() {
    // Arrange
    let root = Path::new("/tmp/config");

    // Act
    let resolved = resolve_glob(root, "*.hcl");

    // Assert
    assert_eq!(resolved, "/tmp/config/*.hcl");
}

#[test]
fn resolve_glob_preserves_subdirectories() {
    // Arrange
    let root = Path::new("/etc/snakeway");

    // Act
    let resolved = resolve_glob(root, "routes/**/*.hcl");

    // Assert
    assert_eq!(resolved, "/etc/snakeway/routes/**/*.hcl");
}
