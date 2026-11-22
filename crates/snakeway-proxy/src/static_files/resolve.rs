use std::path::{Component, Path, PathBuf};

#[derive(Debug)]
pub(crate) enum ResolveError {
    NotFound,
    Forbidden,
    BadPath,
}

#[derive(Debug)]
pub(crate) enum ResolvedStatic {
    File(PathBuf),
    Directory(PathBuf),
}

pub(crate) fn resolve_static_path(
    base_dir: &Path,
    route_prefix: &str,
    request_path: &str,
    index: Option<&str>,
) -> Result<ResolvedStatic, ResolveError> {
    // Sanity checks
    if !request_path.starts_with('/') || !route_prefix.starts_with('/') {
        return Err(ResolveError::BadPath);
    }

    // Ensure the request matches the route prefix
    let rel = request_path
        .strip_prefix(route_prefix)
        .or_else(|| {
            // Allow exact match: "/" route serving "/"
            if route_prefix == "/" {
                Some(request_path)
            } else {
                None
            }
        })
        .ok_or(ResolveError::NotFound)?;

    // Percent-decode ONCE
    let decoded = percent_encoding::percent_decode_str(rel)
        .decode_utf8()
        .map_err(|_| ResolveError::BadPath)?;

    // Strip leading slash after decoding
    let decoded = decoded.trim_start_matches('/');

    // Reject empty path unless an index filename is configured.
    let relative_path = if decoded.is_empty() {
        if let Some(name) = index {
            PathBuf::from(name)
        } else {
            return Err(ResolveError::NotFound);
        }
    } else {
        PathBuf::from(decoded)
    };

    // Path component validation (no traversal, no absolute paths)
    for component in relative_path.components() {
        match component {
            Component::Normal(_) => {}
            Component::CurDir => {}
            _ => {
                // ParentDir, RootDir, Prefix (Windows) — all forbidden
                return Err(ResolveError::Forbidden);
            }
        }
    }

    // Join with base directory
    let full_path = base_dir.join(&relative_path);

    // Canonicalize base dir and target
    let base_canon = base_dir
        .canonicalize()
        .map_err(|_| ResolveError::Forbidden)?;

    let target_canon = match full_path.canonicalize() {
        Ok(p) => p,
        Err(_) => return Err(ResolveError::NotFound),
    };

    // Enforce containment
    if !target_canon.starts_with(&base_canon) {
        return Err(ResolveError::Forbidden);
    }

    // If directory, optionally serve the configured index filename
    if target_canon.is_dir() {
        if let Some(name) = index {
            let index_path = target_canon.join(name);
            if index_path.is_file() {
                return Ok(ResolvedStatic::File(index_path));
            }
        }
        return Ok(ResolvedStatic::Directory(target_canon));
    }

    // Must be a regular file
    if !target_canon.is_file() {
        return Err(ResolveError::NotFound);
    }

    Ok(ResolvedStatic::File(target_canon))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn seeded_dir(files: &[(&str, &str)]) -> (std::path::PathBuf, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        for (name, contents) in files {
            let path = dir.path().join(name);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(path, contents).unwrap();
        }
        (dir.path().to_path_buf(), dir)
    }

    #[test]
    fn a_directory_request_serves_the_configured_index_filename() {
        // Arrange
        let (base, _dir) = seeded_dir(&[("home.htm", "HOMEHTM")]);

        // Act
        let resolved = resolve_static_path(&base, "/", "/", Some("home.htm")).unwrap();

        // Assert
        assert!(matches!(resolved, ResolvedStatic::File(p) if p.ends_with("home.htm")));
    }

    #[test]
    fn a_directory_request_without_an_index_is_not_found() {
        // Arrange
        let (base, _dir) = seeded_dir(&[("home.htm", "HOMEHTM")]);

        // Act
        let resolved = resolve_static_path(&base, "/", "/", None);

        // Assert
        assert!(matches!(resolved, Err(ResolveError::NotFound)));
    }

    #[test]
    fn a_missing_configured_index_in_a_subdirectory_leaves_the_directory() {
        // Arrange
        let (base, _dir) = seeded_dir(&[("sub/home.htm", "HOMEHTM")]);

        // Act
        let resolved = resolve_static_path(&base, "/", "/sub/", Some("index.html")).unwrap();

        // Assert
        assert!(matches!(resolved, ResolvedStatic::Directory(_)));
    }

    #[test]
    fn a_cur_dir_component_in_the_request_stays_inside_the_base() {
        // Arrange
        let (base, _dir) = seeded_dir(&[("a.txt", "A")]);

        // Act
        let resolved = resolve_static_path(&base, "/", "/./a.txt", None).unwrap();

        // Assert
        assert!(matches!(resolved, ResolvedStatic::File(p) if p.ends_with("a.txt")));
    }
}
