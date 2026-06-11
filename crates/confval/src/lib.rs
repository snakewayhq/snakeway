#[cfg(feature = "derive")]
pub use confval_derive::Spec;
#[cfg(feature = "hcl")]
pub mod hcl;
mod issue;
mod origin;
pub mod provenance;
pub mod range;
mod report;
mod severity;
mod validate;

pub use issue::ValidationIssue;
pub use origin::{Origin, SimpleOrigin};
pub use range::RangeConstraint;
pub use report::ValidationReport;
pub use severity::Severity;
pub use validate::ValidateSpec;
