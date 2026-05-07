use crate::types::Origin;
use crate::validation::report::ValidationReportDeprecated;

/// Spec types implement this trait to validate their own field-local invariants.
///
/// Field-local means single-field checks: range validation, format checks,
/// path existence, etc. Cross-field and cross-file checks remain in the
/// centralized validators.
///
/// The `origin` parameter carries the source file location for error messages.
/// It is passed explicitly because nested specs do not carry their own origin.
pub trait ValidateSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReportDeprecated);
}
