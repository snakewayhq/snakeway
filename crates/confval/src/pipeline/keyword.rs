use crate::diagnostic::Report;
use crate::source::Located;

/// A closed set of allowed keyword strings, checked against a located value.
///
/// The spec layer stores keyword fields as `Located<String>` (load-balancing
/// strategies, log levels, fail policies) and validates them against a fixed
/// list, reporting at the value's span with a help line listing the options.
/// `KeywordSet` is that check, factored out so every keyword field reports
/// violations the same way, mirroring [`RangeConstraint`](super::range::RangeConstraint)
/// for numeric bounds.
///
/// The help line is always `expected one of: <comma-joined options>`. If a
/// keyword ever needs custom guidance instead, add a `with_help` constructor at
/// that point.
///
/// ```rust
/// use confval::prelude::{KeywordSet, Located, Report};
///
/// const STRATEGIES: [&str; 2] = ["round_robin", "least_conn"];
/// let mut report = Report::new();
/// KeywordSet::new(&STRATEGIES).check_located(
///     &Located::detached("random".to_string()),
///     "load_balancing_strategy",
///     &mut report,
/// );
/// assert!(report.has_errors());
/// ```
#[derive(Debug, Clone)]
pub struct KeywordSet<'a> {
    pub allowed: &'a [&'a str],
}

impl<'a> KeywordSet<'a> {
    pub const fn new(allowed: &'a [&'a str]) -> Self {
        Self { allowed }
    }

    /// Reports an error unless `value` is one of the allowed keywords. The
    /// message is `unknown {field}: {value}` with a help line of
    /// `expected one of: <comma-joined options>`.
    pub fn check_located(&self, value: &Located<String>, field: &str, report: &mut Report) {
        if self.allowed.contains(&value.value.as_str()) {
            return;
        }
        report
            .error(format!("unknown {field}: {}", value.value))
            .at(value.span)
            .help(format!("expected one of: {}", self.allowed.join(", ")))
            .emit();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const COLORS: [&str; 3] = ["red", "green", "blue"];

    fn check(value: &str, field: &'static str) -> Report {
        let mut report = Report::new();
        KeywordSet::new(&COLORS).check_located(
            &Located::detached(value.to_string()),
            field,
            &mut report,
        );
        report
    }

    #[test]
    fn allowed_keyword_reports_nothing() {
        assert!(!check("red", "color").has_issues());
        assert!(!check("blue", "color").has_issues());
    }

    #[test]
    fn unknown_keyword_reports_with_field_and_value() {
        let report = check("purple", "color");
        assert!(report.has_errors());
        assert_eq!(report.issues()[0].message, "unknown color: purple");
    }

    #[test]
    fn default_help_lists_options() {
        let report = check("purple", "color");
        assert_eq!(
            report.issues()[0].help.as_deref(),
            Some("expected one of: red, green, blue")
        );
    }

    #[test]
    fn error_carries_the_value_span() {
        let mut sources = crate::source::SourceMap::new();
        let id = sources.add("test.hcl", "color = \"purple\"");
        let span = crate::source::Span {
            source: id,
            start: 9,
            end: 15,
        };
        let mut report = Report::new();
        KeywordSet::new(&COLORS).check_located(
            &Located {
                value: "purple".to_string(),
                span,
            },
            "color",
            &mut report,
        );
        assert_eq!(report.issues()[0].span, Some(span));
    }
}
