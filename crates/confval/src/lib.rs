#[cfg(feature = "derive")]
pub use confval_derive::{Config, Spec};
pub mod diagnostic;
pub mod format;
pub mod pipeline;
pub mod source;

pub use pipeline::range::RangeConstraint;

/// The common imports for defining and lowering specs.
///
/// A single `use confval::prelude::*;` pulls the three layers a spec module
/// reaches for: the source-location primitives ([`Located`](source::Located),
/// [`Span`](source::Span)), the diagnostic [`Report`](diagnostic::Report), and
/// the lowering pipeline ([`Lower`](pipeline::Lower),
/// [`Validate`](pipeline::Validate), and the [`narrow`](pipeline::narrow)
/// helpers). Format adapters stay out of the prelude; reach for them through
/// their explicit module path (`confval::format::hcl`).
pub mod prelude {
    pub use crate::RangeConstraint;
    pub use crate::diagnostic::{Issue, IssueBuilder, Report, Severity};
    pub use crate::pipeline::{Lower, LowerAuto, Validate, narrow};
    pub use crate::source::{Located, Source, SourceId, SourceMap, Span};

    #[cfg(feature = "derive")]
    pub use crate::{Config, Spec};
}
