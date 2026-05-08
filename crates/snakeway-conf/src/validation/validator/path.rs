use crate::types::HclOrigin;
use crate::types::device_issues;
use confval::ValidationReport;

pub(crate) fn validate_device_paths(
    paths: &[String],
    report: &mut ValidationReport<HclOrigin>,
    origin: &HclOrigin,
) {
    for path in paths {
        if !path.starts_with('/') {
            report.push(device_issues::device_path_must_start_with_slash(
                path, origin,
            ));
        }
    }
}
