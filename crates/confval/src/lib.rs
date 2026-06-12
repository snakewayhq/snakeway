#[cfg(feature = "derive")]
pub use confval_derive::{Config, Spec};
#[cfg(feature = "hcl")]
pub mod hcl;
pub mod provenance;
pub mod range;
mod severity;

pub use range::RangeConstraint;
pub use severity::Severity;
