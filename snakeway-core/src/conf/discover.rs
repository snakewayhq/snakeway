use crate::conf::validation::error::ConfigError;
use glob::glob;
use std::path::{Path, PathBuf};

/// Discovers files matching a glob pattern.
///
/// Searches the filesystem for all files that match the given glob pattern
/// and returns their paths in sorted order. Invalid paths are silently filtered out.
///
/// # Arguments
///
/// * `pattern` - A glob pattern string (e.g., `"config/**/*.toml"`)
///
/// # Returns
///
/// A sorted `Vec<PathBuf>` of all matching file paths.
///
/// # Errors
///
/// Returns `ConfigError::Glob` if the pattern is malformed or cannot be parsed.
pub fn discover(root: &Path, glob_pattern: &str) -> Result<Vec<PathBuf>, ConfigError> {
    let pattern = &resolve_glob(root, glob_pattern);
    let mut paths: Vec<_> = glob(pattern)
        .map_err(|e| ConfigError::Glob {
            pattern: pattern.to_string(),
            source: e,
        })?
        .filter_map(Result::ok)
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
pub fn resolve_glob(root: &Path, pattern: &str) -> String {
    root.join(pattern).to_string_lossy().into_owned()
}
