use crate::provenance::location::Located;
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

/// The auto-mapping backend for the `Config` derive: infallible unwrapping
/// of `Located` layers when the Spec and Config field share name and inner
/// type. Auto-mapped fields with incompatible types fail as a missing
/// `LowerAuto` implementation, naming both types.
///
/// Numeric narrowing is deliberately absent: the range that makes a cast
/// safe is knowledge this trait does not have, so narrowing always goes
/// through an explicit lowering function.
pub trait LowerAuto<Target> {
    fn lower_auto(&self) -> Target;
}

impl<T: Clone> LowerAuto<T> for Located<T> {
    fn lower_auto(&self) -> T {
        self.value.clone()
    }
}

impl<T: Clone> LowerAuto<Option<T>> for Option<Located<T>> {
    fn lower_auto(&self) -> Option<T> {
        self.as_ref().map(|value| value.value.clone())
    }
}

impl<T: Clone> LowerAuto<Vec<T>> for Vec<Located<T>> {
    fn lower_auto(&self) -> Vec<T> {
        self.iter().map(|value| value.value.clone()).collect()
    }
}

impl<T: Clone> LowerAuto<Vec<T>> for Located<Vec<Located<T>>> {
    fn lower_auto(&self) -> Vec<T> {
        self.value.lower_auto()
    }
}

impl<T: Clone> LowerAuto<Option<Vec<T>>> for Option<Located<Vec<Located<T>>>> {
    fn lower_auto(&self) -> Option<Vec<T>> {
        self.as_ref().map(|list| list.value.lower_auto())
    }
}
