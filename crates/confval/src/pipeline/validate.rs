use crate::diagnostic::Report;

/// Field-local semantic validation for a Spec type.
///
/// A `Validate` impl checks what a value can prove about itself from its own
/// fields: ranges, closed-set keywords, formats, each reported at the span
/// the offending field already carries. Checks that need an enclosing span,
/// cross-field structure, or sibling context do not belong here; they live in
/// the central validators that hold the surrounding `Located` wrappers.
///
/// The trait exists to be named in a bound: `#[derive(Config)]` with the
/// `validate` flag emits `impl Lower<S> ... where S: Validate`, so a spec that
/// can be lowered into a runtime config but has no validator fails to compile.
/// Validation is still invoked explicitly before the lowering gate; the trait
/// guarantees the validator exists, not that lowering calls it.
pub trait Validate {
    fn validate(&self, report: &mut Report);
}
