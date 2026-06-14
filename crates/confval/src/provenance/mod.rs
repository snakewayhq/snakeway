//! Span-first provenance: every parsed value carries its exact location in
//! the source text, so diagnostics can point at the offending value rather
//! than the enclosing section.

mod location;
mod lower;
pub mod narrow;
mod report;
mod source;
mod span;
mod validate;

pub use location::Located;
pub use lower::{Lower, LowerAuto};
pub use report::{Issue, IssueBuilder, Report};
pub use source::{Source, SourceMap};
pub use span::{SourceId, Span};
pub use validate::Validate;

pub use crate::severity::Severity;
