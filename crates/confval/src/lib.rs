#[cfg(feature = "derive")]
pub use confval_derive::{Config, Spec};
mod format;
pub mod provenance;
pub mod range;
mod severity;

pub use range::RangeConstraint;
pub use severity::Severity;
