use crate::conf::types::RuntimeConfig;
use crate::conf::validation::ValidationOutput;

pub struct ValidatedConfig {
    pub config: RuntimeConfig,
    pub validation: ValidationOutput,
}
