use crate::diagnostic::Report;
use crate::source::Located;
use std::fmt;

#[derive(Debug, Clone)]
pub struct RangeConstraint<T> {
    pub min: T,
    pub max: T,
    pub units: Option<&'static str>,
    pub help: Option<&'static str>,
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
            help: None,
        }
    }

    pub const fn with_units(min: T, max: T, units: &'static str) -> Self {
        Self {
            min,
            max,
            units: Some(units),
            help: None,
        }
    }

    /// Checks a located value, pushing a span-carrying issue to the report
    /// if it is out of range. Message and help text are identical to
    /// [`check`](Self::check), so both report worlds describe violations the
    /// same way.
    pub fn check_located(&self, value: &Located<T>, field: &'static str, report: &mut Report) {
        let (limit, kind) = if value.value < self.min {
            (self.min, "at least")
        } else if value.value > self.max {
            (self.max, "at most")
        } else {
            return;
        };
        let help = self.help.map(String::from).unwrap_or_else(|| {
            format!(
                "Set {} to {} {}{}",
                field,
                kind,
                limit,
                self.units.unwrap_or("")
            )
        });
        report
            .error(format!("{} must be {} {}", field, kind, limit))
            .at(value.span)
            .help(help)
            .emit();
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
/// range_constraint!(WORKERS, usize, min: 1, max: 128, help: "Match this to your CPU core count.");
/// ```
#[macro_export]
macro_rules! range_constraint {
    ($name:ident, $T:ty, min: $min:expr, max: $max:expr, help: $help:literal) => {
        const $name: RangeConstraint<$T> = $crate::RangeConstraint {
            min: $min,
            max: $max,
            units: None,
            help: Some($help),
        };
    };
    ($name:ident, $T:ty, min: $min:expr, max: $max:expr, units: $units:literal, help: $help:literal) => {
        const $name: RangeConstraint<$T> = $crate::RangeConstraint {
            min: $min,
            max: $max,
            units: Some($units),
            help: Some($help),
        };
    };
    ($name:ident, $T:ty, min: $min:expr, max: $max:expr $(, units: $units:literal)?) => {
        const $name: RangeConstraint<$T> = $crate::RangeConstraint {
            min: $min,
            max: $max,
            units: range_constraint!(@units $($units)?),
            help: None,
        };
    };
    (@units $units:literal) => { Some($units) };
    (@units) => { None };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provenance::{Located, Report};

    range_constraint!(PORT, i64, min: 1, max: 65535);
    range_constraint!(THREADS, i64, min: 1, max: 1024);
    range_constraint!(INTERVAL, i64, min: 1, max: 3600, units: "s");
    range_constraint!(WORKERS, i64, min: 1, max: 128, help: "Match this to your CPU core count.");
    range_constraint!(TIMEOUT, i64, min: 1, max: 300, units: "s", help: "Keep this under 5 minutes for responsive shutdowns.");

    fn check(constraint: &RangeConstraint<i64>, value: i64, field: &'static str) -> Report {
        let mut report = Report::new();
        constraint.check_located(&Located::detached(value), field, &mut report);
        report
    }

    #[test]
    fn in_range_reports_nothing() {
        assert!(!check(&PORT, 80, "port").has_issues());
        assert!(!check(&PORT, 1, "port").has_issues());
        assert!(!check(&PORT, 65535, "port").has_issues());
    }

    #[test]
    fn below_min_reports_issue() {
        let report = check(&PORT, 0, "port");
        assert!(
            report.issues()[0]
                .message
                .contains("port must be at least 1")
        );
    }

    #[test]
    fn above_max_reports_issue() {
        let report = check(&THREADS, 2000, "threads");
        assert!(
            report.issues()[0]
                .message
                .contains("threads must be at most 1024")
        );
    }

    #[test]
    fn help_includes_units_when_present() {
        let report = check(&INTERVAL, 0, "interval");
        assert!(report.issues()[0].help.as_ref().unwrap().contains("s"));
    }

    #[test]
    fn custom_help_overrides_generated_text() {
        let report = check(&WORKERS, 0, "workers");
        assert_eq!(
            report.issues()[0].help.as_deref(),
            Some("Match this to your CPU core count.")
        );
        let report = check(&TIMEOUT, 0, "timeout");
        assert_eq!(
            report.issues()[0].help.as_deref(),
            Some("Keep this under 5 minutes for responsive shutdowns.")
        );
    }
}
