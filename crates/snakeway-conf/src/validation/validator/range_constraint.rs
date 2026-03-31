use crate::types::Origin;
use crate::validation::ValidationReport;

#[derive(Debug, Clone)]
pub(crate) struct RangeConstraint<T> {
    pub(crate) min: T,
    pub(crate) max: T,
    pub(crate) units: Option<&'static str>,
}

impl<T> RangeConstraint<T>
where
    T: PartialOrd + std::fmt::Display + Copy,
{
    /// Validate that the given value is within the range of this constraint.
    /// Invalid values are reported as errors in the validation report.
    pub(crate) fn validate(
        &self,
        value: T,
        field: &'static str,
        report: &mut ValidationReport,
        origin: &Origin,
    ) {
        if value < self.min || value > self.max {
            let units = self.units.unwrap_or("");
            report.error(
                format!(
                    "invalid {}: {}{} (must be between {}{} and {}{})",
                    field, value, units, self.min, units, self.max, units
                ),
                origin,
                None,
            );
        }
    }
}

macro_rules! range_constraint {
    ($name:ident, $T:ty, min: $min:expr, max: $max:expr $(, units: $units:literal)?) => {
        const $name: RangeConstraint<$T> = RangeConstraint {
            min: $min,
            max: $max,
            units: range_constraint!(@units $($units)?),
        };
    };
    (@units $units:literal) => { Some($units) };
    (@units) => { None };
}

pub(crate) use range_constraint;

macro_rules! validate_range_field {
    ($constraint:expr, $self:ident . $field:ident, $report:expr, $origin:expr) => {
        $constraint.validate($self.$field, stringify!($field), $report, $origin);
    };
    ($constraint:expr, $var:ident, $report:expr, $origin:expr) => {
        $constraint.validate($var, stringify!($var), $report, $origin);
    };
}

pub(crate) use validate_range_field;
