use crate::types::Origin;
use crate::validation::ValidationReport;
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
    report: &mut ValidationReport,
    origin: &Origin,
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
    report: &mut ValidationReport,
    origin: &Origin,
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
