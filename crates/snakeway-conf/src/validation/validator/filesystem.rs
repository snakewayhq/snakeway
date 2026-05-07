use crate::types::OriginDeprecated;
use crate::validation::ValidationReportDeprecated;
use nix::NixPath;
use std::fs;
use std::path::Path;

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

pub(crate) fn validate_geoip_db_file(
    geoip_db: &Path,
    report: &mut ValidationReportDeprecated,
    origin: &OriginDeprecated,
) {
    if !geoip_db.is_file() {
        if NixPath::is_empty(geoip_db) {
            report.geoip_db_path_is_empty(geoip_db.display(), origin);
        }
        if !geoip_db.exists() {
            report.geoip_db_path_does_not_exist(geoip_db.display(), origin);
        }
        if !geoip_db.is_file() {
            report.geoip_db_is_not_a_file(geoip_db.display(), origin);
        }
    }
}

pub(crate) fn validate_ua_parser_regexes_file(
    path: &Path,
    report: &mut ValidationReportDeprecated,
    origin: &OriginDeprecated,
) {
    if NixPath::is_empty(path) {
        report.ua_parser_regexes_path_is_empty(path.display(), origin);
        return;
    }
    if !path.exists() {
        report.ua_parser_regexes_path_does_not_exist(path.display(), origin);
        return;
    }
    if !path.is_file() {
        report.ua_parser_regexes_path_is_not_a_file(path.display(), origin);
        return;
    }
    if let Ok(contents) = std::fs::read_to_string(path)
        && !contents.contains("user_agent_parsers")
    {
        report.ua_parser_regexes_file_missing_expected_content(path.display(), origin);
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
        let mut report = ValidationReportDeprecated::default();
        let origin = OriginDeprecated::test("ua_parser");
        let path = Path::new("");

        // Act
        validate_ua_parser_regexes_file(path, &mut report, &origin);

        // Assert
        assert!(report.has_violations());
        assert!(report.errors.iter().any(|e| e.message.contains("empty")));
    }

    #[test]
    fn validate_ua_parser_not_found() {
        // Arrange
        let mut report = ValidationReportDeprecated::default();
        let origin = OriginDeprecated::test("ua_parser");
        let path = Path::new("/nonexistent/regexes.yaml");

        // Act
        validate_ua_parser_regexes_file(path, &mut report, &origin);

        // Assert
        assert!(report.has_violations());
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.message.contains("does not exist"))
        );
    }

    #[test]
    fn validate_ua_parser_is_directory() {
        // Arrange
        let dir = tempdir().expect("failed to create temp dir");
        let mut report = ValidationReportDeprecated::default();
        let origin = OriginDeprecated::test("ua_parser");

        // Act
        validate_ua_parser_regexes_file(dir.path(), &mut report, &origin);

        // Assert
        assert!(report.has_violations());
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.message.contains("not a file"))
        );
    }

    #[test]
    fn validate_ua_parser_missing_content() {
        // Arrange
        let dir = tempdir().expect("failed to create temp dir");
        let path = dir.path().join("regexes.yaml");
        let mut f = File::create(&path).expect("failed to create file");
        f.write_all(b"some random content")
            .expect("failed to write");
        let mut report = ValidationReportDeprecated::default();
        let origin = OriginDeprecated::test("ua_parser");

        // Act
        validate_ua_parser_regexes_file(&path, &mut report, &origin);

        // Assert
        assert!(report.has_violations());
        assert!(
            report
                .warnings
                .iter()
                .any(|w| w.message.contains("does not appear to be a valid"))
        );
    }

    #[test]
    fn validate_ua_parser_valid() {
        // Arrange
        let dir = tempdir().expect("failed to create temp dir");
        let path = dir.path().join("regexes.yaml");
        let mut f = File::create(&path).expect("failed to create file");
        f.write_all(b"user_agent_parsers:\n  - regex: '.*'\n")
            .expect("failed to write");
        let mut report = ValidationReportDeprecated::default();
        let origin = OriginDeprecated::test("ua_parser");

        // Act
        validate_ua_parser_regexes_file(&path, &mut report, &origin);

        // Assert
        assert!(!report.has_violations());
    }
}
