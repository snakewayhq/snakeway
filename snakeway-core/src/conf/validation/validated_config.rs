use crate::conf::types::RuntimeConfig;
use crate::conf::validation::report::ValidationReport;

pub struct ValidatedConfig {
    pub config: RuntimeConfig,
    pub validation_report: ValidationReport,
}
