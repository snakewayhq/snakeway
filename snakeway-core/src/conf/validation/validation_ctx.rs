use crate::conf::validation::error::ConfigError;

#[derive(Default)]
pub struct ValidationCtx {
    errors: Vec<ConfigError>,
}

impl ValidationCtx {
    pub fn push(&mut self, err: ConfigError) {
        self.errors.push(err);
    }

    pub fn into_result(self) -> Result<(), ValidationErrors> {
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(ValidationErrors(self.errors))
        }
    }
}

use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
#[error("configuration validation failed")]
pub struct ValidationErrors(#[related] pub Vec<ConfigError>);
