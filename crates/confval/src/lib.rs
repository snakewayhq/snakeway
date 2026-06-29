#[cfg(feature = "derive")]
pub use confval_derive::{Config, Spec};
pub mod diagnostic;
pub mod format;
pub mod pipeline;
pub mod source;

pub use pipeline::keyword::KeywordSet;
pub use pipeline::range::RangeConstraint;

/// The common imports for defining and lowering specs.
///
/// A single `use confval::prelude::*;` pulls the everyday names a spec module
/// reaches for: the source-location primitives ([`Located`](source::Located),
/// [`Span`](source::Span), [`SourceMap`](source::SourceMap)), the diagnostic
/// [`Report`](diagnostic::Report), the lowering pipeline
/// ([`Lower`](pipeline::Lower), [`Validate`](pipeline::Validate), and the
/// [`narrow`](pipeline::narrow) helpers), the constraint validators
/// ([`KeywordSet`], [`RangeConstraint`] and its [`range_constraint!`] macro),
/// and, with the `derive` feature, the [`Spec`] and [`Config`] derives.
/// `RangeConstraint` and its macro travel together so the validated-range
/// pattern works from one import.
///
/// Format adapters stay out of the prelude; reach for them through their
/// explicit module path (`confval::format::hcl`). The diagnostic internals
/// ([`Issue`](diagnostic::Issue), [`Severity`](diagnostic::Severity)) and the
/// remaining source types ([`Source`](source::Source),
/// [`SourceId`](source::SourceId)) likewise stay behind their module paths.
pub mod prelude {
    pub use crate::diagnostic::Report;
    pub use crate::pipeline::{Lower, Validate, narrow};
    pub use crate::source::{Located, SourceMap, Span};
    pub use crate::{KeywordSet, RangeConstraint, range_constraint};

    #[cfg(feature = "derive")]
    pub use crate::{Config, Spec};
}
