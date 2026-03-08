use crate::conf::types::RuntimeConfig;
use crate::conf::validation::report::ValidationReport;

pub(crate) struct ValidatedConfig {
    pub(crate) config: RuntimeConfig,
    pub(crate) validation_report: ValidationReport,
}

impl ValidatedConfig {
    pub(crate) fn is_valid(&self) -> bool {
        self.validation_report.errors.is_empty()
    }
}
