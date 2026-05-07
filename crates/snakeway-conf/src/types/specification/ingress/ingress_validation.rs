use crate::types::{IngressSpec, OriginDeprecated};
use crate::validation::{ValidateSpec, ValidationReportDeprecated};

impl ValidateSpec for IngressSpec {
    fn validate(&self, _origin: &OriginDeprecated, report: &mut ValidationReportDeprecated) {
        // Bind validation.
        if let Some(bind) = &self.bind {
            bind.validate(&bind.origin, report);
        }

        // Admin bind validation.
        if let Some(bind_admin) = &self.bind_admin {
            bind_admin.validate(&bind_admin.origin, report);
        }

        // Static files validation.
        self.static_files
            .iter()
            .for_each(|static_files| static_files.validate(&static_files.origin, report));

        // Services validation.
        self.services
            .iter()
            .for_each(|service| service.validate(&service.origin, report));
    }
}
