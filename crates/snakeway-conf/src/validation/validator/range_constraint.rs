use crate::types::Origin;
use crate::validation::ValidationReport;

#[derive(Debug, Clone)]
pub(crate) struct RangeConstraint<T> {
    pub(crate) min: T,
    pub(crate) max: T,
    pub(crate) label: &'static str,
    pub(crate) units: Option<&'static str>,
}

impl<T> RangeConstraint<T>
where
    T: PartialOrd + std::fmt::Display + Copy,
{
    /// Validate that the given value is within the range of this constraint.
    /// Invalid values are reported as errors in the validation report.
    pub(crate) fn validate(&self, value: T, report: &mut ValidationReport, origin: &Origin) {
        if value < self.min || value > self.max {
            let units = self.units.unwrap_or("");
            report.error(
                format!(
                    "invalid {}: {}{} (must be between {}{} and {}{})",
                    self.label, value, units, self.min, units, self.max, units
                ),
                origin,
                None,
            );
        }
    }
}

macro_rules! range_constraint {
    ($name:ident, $T:ty, min: $min:expr, max: $max:expr, label: $label:literal $(, units: $units:literal)?) => {
        const $name: RangeConstraint<$T> = RangeConstraint {
            min: $min,
            max: $max,
            label: $label,
            units: range_constraint!(@units $($units)?),
        };
    };
    (@units $units:literal) => { Some($units) };
    (@units) => { None };
}

pub(crate) use range_constraint;
