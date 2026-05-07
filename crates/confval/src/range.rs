use crate::{Origin, ValidationIssue};
use std::fmt;

#[derive(Debug, Clone)]
pub struct RangeConstraint<T> {
    pub min: T,
    pub max: T,
    pub units: Option<&'static str>,
}

impl<T> RangeConstraint<T>
where
    T: PartialOrd + fmt::Display + Copy,
{
    pub const fn new(min: T, max: T) -> Self {
        Self {
            min,
            max,
            units: None,
        }
    }

    pub const fn with_units(min: T, max: T, units: &'static str) -> Self {
        Self {
            min,
            max,
            units: Some(units),
        }
    }

    /// Returns `Some(issue)` if value is out of range, `None` if valid.
    pub fn check<O: Origin>(
        &self,
        value: T,
        field: &'static str,
        origin: &O,
    ) -> Option<ValidationIssue<O>> {
        if value < self.min {
            Some(ValidationIssue::error_with_help(
                format!("{} must be at least {}", field, self.min),
                origin.clone(),
                format!(
                    "Set {} to at least {}{}",
                    field,
                    self.min,
                    self.units.unwrap_or("")
                ),
            ))
        } else if value > self.max {
            Some(ValidationIssue::error_with_help(
                format!("{} must be at most {}", field, self.max),
                origin.clone(),
                format!(
                    "Set {} to at most {}{}",
                    field,
                    self.max,
                    self.units.unwrap_or("")
                ),
            ))
        } else {
            None
        }
    }
}

/// Define a named range constraint as a const.
///
/// ```rust
/// use confval::{RangeConstraint, range_constraint};
///
/// range_constraint!(THREADS, usize, min: 1, max: 1024);
/// range_constraint!(PORT, u16, min: 1, max: 65535);
/// range_constraint!(INTERVAL, u64, min: 1, max: 3600, units: "s");
/// ```
#[macro_export]
macro_rules! range_constraint {
    ($name:ident, $T:ty, min: $min:expr, max: $max:expr $(, units: $units:literal)?) => {
        const $name: RangeConstraint<$T> = $crate::RangeConstraint {
            min: $min,
            max: $max,
            units: range_constraint!(@units $($units)?),
        };
    };
    (@units $units:literal) => { Some($units) };
    (@units) => { None };
}

/// Validate a field against a range constraint, pushing any issue to the report.
/// Uses `stringify!` to derive the field name at zero runtime cost.
///
/// ```rust,ignore
/// validate_range_field!(THREADS, self.threads, report, origin);
/// validate_range_field!(PORT, port, report, origin);
/// ```
#[macro_export]
macro_rules! validate_range_field {
    ($constraint:expr, $self:ident . $field:ident, $report:expr, $origin:expr) => {
        if let Some(issue) =
            $crate::RangeConstraint::check(&$constraint, $self.$field, stringify!($field), $origin)
        {
            $report.error(issue);
        }
    };
    ($constraint:expr, $var:ident, $report:expr, $origin:expr) => {
        if let Some(issue) =
            $crate::RangeConstraint::check(&$constraint, $var, stringify!($var), $origin)
        {
            $report.error(issue);
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SimpleOrigin;

    fn test_origin() -> SimpleOrigin {
        SimpleOrigin::new("test.toml", "server block")
    }

    range_constraint!(PORT, u16, min: 1, max: 65535);
    range_constraint!(THREADS, usize, min: 1, max: 1024);
    range_constraint!(INTERVAL, u64, min: 1, max: 3600, units: "s");

    #[test]
    fn in_range_returns_none() {
        assert!(PORT.check(80, "port", &test_origin()).is_none());
        assert!(PORT.check(1, "port", &test_origin()).is_none());
        assert!(PORT.check(65535, "port", &test_origin()).is_none());
    }

    #[test]
    fn below_min_returns_issue() {
        let issue = PORT.check(0, "port", &test_origin()).unwrap();
        assert!(issue.message.contains("port must be at least 1"));
    }

    #[test]
    fn above_max_returns_issue() {
        let issue = THREADS.check(2000, "threads", &test_origin()).unwrap();
        assert!(issue.message.contains("threads must be at most 1024"));
    }

    #[test]
    fn help_includes_units_when_present() {
        let issue = INTERVAL.check(0, "interval", &test_origin()).unwrap();
        assert!(issue.help.as_ref().unwrap().contains("s"));
    }

    #[test]
    fn help_has_no_units_when_absent() {
        let issue = PORT.check(0, "port", &test_origin()).unwrap();
        let help = issue.help.as_ref().unwrap();
        assert!(
            !help.ends_with("s"),
            "expected no trailing units, got: {help}"
        );
    }
}
