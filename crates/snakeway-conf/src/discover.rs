use crate::validation::ConfigError;
use glob::glob;
use std::path::{Path, PathBuf};

/// Discovers files matching a glob pattern.
///
/// Searches the filesystem for all files that match the given glob pattern
/// and returns their paths in sorted order. Invalid paths are silently filtered out.
///
/// # Arguments
///
/// * `pattern` - A glob pattern string (e.g., `"config/**/*.hcl"`)
///
/// # Returns
///
/// A sorted `Vec<PathBuf>` of all matching file paths.
///
/// # Errors
///
/// Returns `ConfigError::Glob` if the pattern is malformed or cannot be parsed.
pub(crate) fn discover(root: &Path, glob_pattern: &str) -> Result<Vec<PathBuf>, ConfigError> {
    let pattern = &resolve_glob(root, glob_pattern)?;
    let mut paths: Vec<_> = glob(pattern)
        .map_err(|e| ConfigError::Glob {
            pattern: pattern.to_string(),
            source: e,
        })?
        .filter_map(Result::ok)
        .filter(|p| p.is_file())
        .collect();

    paths.sort();
    Ok(paths)
}
/// Resolves a glob pattern relative to a root directory.
///
/// Joins the given `pattern` to the `root` path and returns it as a string.
/// This is useful for constructing absolute glob patterns from a base directory
/// and a relative pattern.
///
/// # Arguments
///
/// * `root` - The base directory path to resolve the pattern against
/// * `pattern` - The glob pattern to append to the root path
///
/// # Returns
///
/// A `String` containing the resolved absolute path pattern
pub(crate) fn resolve_glob(root: &Path, pattern: &str) -> Result<String, ConfigError> {
    let joined = root.join(pattern);

    if let Some(s) = joined.to_str() {
        return Ok(s.to_owned());
    }

    Err(ConfigError::ResolveGlob {
        root: root.to_string_lossy().into_owned(),
        pattern: pattern.to_string(),
        os_string: joined.into_os_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let resolved = resolve_glob(root, "*.hcl").expect("resolve_glob should not fail");

        // Assert
        assert_eq!(resolved, "/tmp/config/*.hcl");
    }

    #[test]
    fn resolve_glob_preserves_subdirectories() {
        // Arrange
        let root = Path::new("/etc/snakeway");

        // Act
        let resolved = resolve_glob(root, "routes/**/*.hcl").expect("resolve_glob should not fail");

        // Assert
        assert_eq!(resolved, "/etc/snakeway/routes/**/*.hcl");
    }
}
