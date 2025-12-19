use std::path::{Component, Path, PathBuf};

#[derive(Debug)]
pub enum ResolveError {
    NotFound,
    Forbidden,
    BadPath,
}

#[derive(Debug)]
pub enum ResolvedStatic {
    File(PathBuf),
    Directory(PathBuf),
}

pub fn resolve_static_path(
    base_dir: &Path,
    route_prefix: &str,
    request_path: &str,
    index: bool,
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

    // Reject empty path unless index is allowed
    let relative_path = if decoded.is_empty() {
        if index {
            PathBuf::from("index.html")
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

    // If directory, optionally append index.html
    if target_canon.is_dir() {
        if index {
            let index_path = target_canon.join("index.html");
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
