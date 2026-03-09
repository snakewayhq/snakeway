use crate::conf::types::RuntimeConfig;
use crate::conf::validation::report::ValidationReport;

pub struct ValidatedConfig {
    pub config: RuntimeConfig,
    pub validation_report: ValidationReport,
}

impl ValidatedConfig {
    pub fn is_valid(&self) -> bool {
        self.validation_report.errors.is_empty()
    }
}
