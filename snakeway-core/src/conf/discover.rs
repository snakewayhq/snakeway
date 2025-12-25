use crate::conf::error::ConfigError;
use glob::glob;
use std::path::PathBuf;

pub fn discover(pattern: &str) -> Result<Vec<PathBuf>, ConfigError> {
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
