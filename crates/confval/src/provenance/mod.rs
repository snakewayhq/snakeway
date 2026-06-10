//! Span-first provenance: every parsed value carries its exact location in
//! the source text, so diagnostics can point at the offending value rather
//! than the enclosing section.

mod location;
mod lower;
mod report;
mod source;
mod span;

pub use location::Located;
pub use lower::Lower;
pub use report::{Issue, IssueBuilder, Report};
pub use source::{Source, SourceMap};
pub use span::{SourceId, Span};

pub use crate::severity::Severity;
