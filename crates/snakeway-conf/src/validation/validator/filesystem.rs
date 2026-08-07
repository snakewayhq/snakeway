use confval::prelude::{Located, Report};
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn read_nonempty_file(path: &Path) -> Result<Vec<u8>, String> {
    if !path.is_file() {
        return Err(format!("file does not exist: {}", path.display()));
    }

    let bytes = fs::read(path).map_err(|e| format!("cannot read file {}: {e}", path.display()))?;

    if bytes.is_empty() {
        return Err(format!("file is empty: {}", path.display()));
    }

    Ok(bytes)
}

/// Report an error unless `located` points at an existing regular file.
/// At most one diagnostic is reported. The return value says whether the path is
/// usable, so a caller doing further checks, such as reading the file, can bail.
/// `help`, when given, is attached to whichever error fires
/// (empty / missing / not-a-file), so a caller can offer remediation specific
/// to its setting.
pub(crate) fn require_existing_file(
    located: &Located<PathBuf>,
    label: &str,
    help: Option<&str>,
    report: &mut Report,
) -> bool {
    let span = located.span;
    let emit = |report: &mut Report, message: String| {
        let issue = report.error(message).at(span);
        match help {
            Some(help) => issue.help(help).emit(),
            None => issue.emit(),
        }
    };

    let path = located.value.as_path();
    if path.as_os_str().is_empty() {
        emit(report, format!("{label} path is empty: {}", path.display()));
        return false;
    }
    if !path.exists() {
        emit(
            report,
            format!("{label} path does not exist: {}", path.display()),
        );
        return false;
    }
    if !path.is_file() {
        emit(
            report,
            format!("{label} path is not a file: {}", path.display()),
        );
        return false;
    }
    true
}

/// Report an error unless `located` points at an existing directory.
pub(crate) fn require_existing_dir(
    located: &Located<PathBuf>,
    label: &str,
    report: &mut Report,
) -> bool {
    let path = located.value.as_path();
    if path.as_os_str().is_empty() {
        report
            .error(format!("{label} path is required"))
            .at(located.span)
            .emit();
        return false;
    }
    if !path.is_dir() {
        report
            .error(format!(
                "{label} does not exist or is not a directory: {}",
                path.display()
            ))
            .at(located.span)
            .emit();
        return false;
    }
    true
}

pub(crate) fn validate_geoip_db_file(geoip_db: &Located<PathBuf>, report: &mut Report) {
    require_existing_file(
        geoip_db,
        "geoip db",
        Some("Provide a path to a MaxMind .mmdb database file (for example, GeoLite2-City.mmdb)."),
        report,
    );
}

pub(crate) fn validate_ua_parser_regexes_file(located: &Located<PathBuf>, report: &mut Report) {
    if !require_existing_file(
        located,
        "ua_parser_regexes",
        Some(
            "Provide a valid path to a ua-parser regexes.yaml file, or remove the setting \
             to use the bundled default.",
        ),
        report,
    ) {
        return;
    }
    let path = located.value.as_path();
    if let Ok(contents) = std::fs::read_to_string(path)
        && !contents.contains("user_agent_parsers")
    {
        report
            .error(format!(
                "ua_parser_regexes file does not appear to be a valid ua-parser regexes.yaml: {}",
                path.display()
            ))
            .at(located.span)
            .help(
                "Expected the file to contain a 'user_agent_parsers' section. See \
                 https://github.com/ua-parser/uap-core for the expected format.",
            )
            .emit();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn read_nonempty_file_not_found() {
        // Arrange
        let path = PathBuf::from("/nonexistent/file.txt");

        // Act
        let result = read_nonempty_file(&path);

        // Assert
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("file does not exist"));
    }

    #[test]
    fn read_nonempty_file_empty() {
        // Arrange
        let dir = tempdir().expect("failed to create temp dir");
        let path = dir.path().join("empty.txt");
        File::create(&path).expect("failed to create empty file");

        // Act
        let result = read_nonempty_file(&path);

        // Assert
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("file is empty"));
    }

    #[test]
    fn read_nonempty_file_valid() {
        // Arrange
        let dir = tempdir().expect("failed to create temp dir");
        let path = dir.path().join("valid.txt");
        let mut f = File::create(&path).expect("failed to create file");
        f.write_all(b"hello world").expect("failed to write");

        // Act
        let result = read_nonempty_file(&path);

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), b"hello world");
    }

    #[test]
    fn read_nonempty_file_directory() {
        // Arrange
        let dir = tempdir().expect("failed to create temp dir");

        // Act
        let result = read_nonempty_file(dir.path());

        // Assert
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("file does not exist"));
    }

    #[test]
    fn validate_ua_parser_empty_path() {
        // Arrange
        let mut report = Report::new();
        let path = Located::detached(PathBuf::from(""));

        // Act
        validate_ua_parser_regexes_file(&path, &mut report);

        // Assert
        assert!(report.has_issues());
        // The empty-path case also carries the remediation help (dev only
        // attached it to the does-not-exist case).
        assert!(report.issues().iter().any(|e| {
            e.message.contains("empty")
                && e.help
                    .as_deref()
                    .is_some_and(|h| h.contains("bundled default"))
        }));
    }

    #[test]
    fn validate_ua_parser_not_found() {
        // Arrange
        let mut report = Report::new();
        let path = Located::detached(PathBuf::from("/nonexistent/regexes.yaml"));

        // Act
        validate_ua_parser_regexes_file(&path, &mut report);

        // Assert
        assert!(report.has_issues());
        assert!(report.issues().iter().any(|e| {
            e.message.contains("does not exist")
                && e.help
                    .as_deref()
                    .is_some_and(|h| h.contains("bundled default"))
        }));
    }

    #[test]
    fn validate_ua_parser_is_directory() {
        // Arrange
        let mut report = Report::new();
        let dir = tempdir().expect("failed to create temp dir");
        let path = Located::detached(dir.path().to_path_buf());

        // Act
        validate_ua_parser_regexes_file(&path, &mut report);

        // Assert
        assert!(report.has_issues());
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("not a file"))
        );
    }

    #[test]
    fn validate_ua_parser_missing_content() {
        // Arrange
        let mut report = Report::new();
        let dir = tempdir().expect("failed to create temp dir");
        let file_path = dir.path().join("regexes.yaml");
        let mut file = File::create(&file_path).expect("create file");
        file.write_all(b"something else entirely").expect("write");
        let path = Located::detached(file_path);

        // Act
        validate_ua_parser_regexes_file(&path, &mut report);

        // Assert
        assert!(report.has_issues());
        assert!(
            report
                .issues()
                .iter()
                .any(|w| w.message.contains("does not appear to be a valid"))
        );
    }

    #[test]
    fn validate_ua_parser_valid() {
        // Arrange
        let mut report = Report::new();
        let dir = tempdir().expect("failed to create temp dir");
        let file_path = dir.path().join("regexes.yaml");
        let mut file = File::create(&file_path).expect("create file");
        file.write_all(b"user_agent_parsers:\n  - regex: test")
            .expect("write");
        let path = Located::detached(file_path);

        // Act
        validate_ua_parser_regexes_file(&path, &mut report);

        // Assert
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
    }

    #[test]
    fn validate_geoip_db_missing_file() {
        // Arrange
        let mut report = Report::new();
        let path = Located::detached(PathBuf::from("/nonexistent/db.mmdb"));

        // Act
        validate_geoip_db_file(&path, &mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("geoip db path does not exist"))
        );
    }
}
