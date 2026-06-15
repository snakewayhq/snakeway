#[cfg(feature = "derive")]
pub use confval_derive::{Config, Spec};
pub mod diagnostic;
mod format;
pub mod provenance;
pub mod range;
mod severity;
pub mod source;

pub use range::RangeConstraint;
pub use severity::Severity;
