use crate::conf::validation::ConfigError;
use crate::conf::validation::ConfigWarning;
use miette::Diagnostic;
use thiserror::Error;

#[derive(Default)]
pub struct ValidationCtx {
    errors: Vec<ConfigError>,
    warnings: Vec<ConfigWarning>,
}

impl ValidationCtx {
    pub fn error(&mut self, err: ConfigError) {
        self.errors.push(err);
    }

    pub fn warn(&mut self, warn: ConfigWarning) {
        self.warnings.push(warn);
    }

    pub fn into_result(self) -> Result<ValidationOutput, ValidationErrors> {
        if self.errors.is_empty() {
            Ok(ValidationOutput {
                warnings: self.warnings,
            })
        } else {
            Err(ValidationErrors {
                errors: self.errors,
                warnings: self.warnings,
            })
        }
    }
}

#[derive(Debug, Error, Diagnostic)]
#[error("configuration validation failed")]
pub struct ValidationErrors {
    #[related]
    pub errors: Vec<ConfigError>,

    #[related]
    pub warnings: Vec<ConfigWarning>,
}

pub struct ValidationOutput {
    pub warnings: Vec<ConfigWarning>,
}
