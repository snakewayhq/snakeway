use crate::types::Origin;
use crate::validation::ValidationReportDeprecated;

pub(crate) fn validate_device_paths(
    paths: &[String],
    report: &mut ValidationReportDeprecated,
    origin: &Origin,
) {
    for path in paths {
        if !path.starts_with('/') {
            report.device_path_must_start_with_slash(path, origin);
        }
    }
}
