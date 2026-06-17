//! Span-first provenance: every parsed value carries its exact location in
//! the source text, so diagnostics can point at the offending value rather
//! than the enclosing section.

mod lower;
pub mod narrow;

pub mod range;
mod validate;

pub use lower::{Lower, LowerAuto};

pub use validate::Validate;
