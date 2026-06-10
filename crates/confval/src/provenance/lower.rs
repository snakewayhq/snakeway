use crate::provenance::report::Report;

/// Conversion from a Spec type to its runtime Config type.
///
/// The Spec is taken by reference because Specs are retained for history and
/// diffing. Failure returns `None` with the explanation already pushed to
/// the report.
///
/// Callers must gate on [`Report::has_errors`](crate::provenance::Report::has_errors)
/// before lowering: lowering functions may assume field-level validation
/// passed, which is what makes narrowing conversions safe to write.
pub trait Lower<S>: Sized {
    fn lower(spec: &S, report: &mut Report) -> Option<Self>;
}
