//! Checked integer narrowing for lowering functions.
//!
//! Spec types store every integer as `i64` (the widest type the source
//! format produces); runtime types use the exact width they need. These
//! helpers narrow between the two and are shaped to slot directly into
//! `#[confval(lower(from = ..., with = ...))]`.
//!
//! Lowering runs only after the error gate, so a value that does not fit
//! means a validation rule is missing, not that the operator made a typo.
//! Rather than truncating silently, the helpers report the failure at the
//! value's span and return `None`.

use crate::diagnostic::Report;
use crate::source::Located;
use std::time::Duration;

macro_rules! narrow_fns {
    ($plain:ident, $opt:ident, $target:ty) => {
        /// Narrow a located `i64` to the target width, reporting at the
        /// value's span if it does not fit.
        pub fn $plain(value: &Located<i64>, report: &mut Report) -> Option<$target> {
            match <$target>::try_from(value.value) {
                Ok(narrowed) => Some(narrowed),
                Err(_) => {
                    report
                        .error(format!(
                            "value {} is out of range for {}",
                            value.value,
                            stringify!($target)
                        ))
                        .at(value.span)
                        .emit();
                    None
                }
            }
        }

        /// Optional-field variant: `None` in, `Some(None)` out. The outer
        /// `Option` is the failure channel.
        pub fn $opt(value: &Option<Located<i64>>, report: &mut Report) -> Option<Option<$target>> {
            match value {
                Some(value) => $plain(value, report).map(Some),
                None => Some(None),
            }
        }
    };
}

narrow_fns!(i64_to_u16, opt_i64_to_u16, u16);
narrow_fns!(i64_to_u32, opt_i64_to_u32, u32);
narrow_fns!(i64_to_u64, opt_i64_to_u64, u64);
narrow_fns!(i64_to_usize, opt_i64_to_usize, usize);

/// Widen a located `i64` to `f64`. This is infallible (the `report` argument is
/// unused), but it lives here so a `#[confval(lower(with = ...))]` attribute,
/// which cannot hold a bare `as` cast, has a function to name. Values above
/// 2^53 lose integer precision, which is harmless for the ratios and rates this
/// is used for.
pub fn i64_to_f64(value: &Located<i64>, _report: &mut Report) -> Option<f64> {
    Some(value.value as f64)
}

/// Convert a located `i64` count of seconds to a `Duration`, reporting at the
/// value's span if it is negative (out of range for `u64`). This routes the
/// conversion through the same checked narrow as the integer helpers, so a
/// negative duration is rejected rather than wrapping into a near-unbounded one.
pub fn i64_secs_to_duration(value: &Located<i64>, report: &mut Report) -> Option<Duration> {
    i64_to_u64(value, report).map(Duration::from_secs)
}

/// Optional-field variant of [`i64_secs_to_duration`]: `None` in, `Some(None)`
/// out. The outer `Option` is the failure channel.
pub fn opt_i64_secs_to_duration(
    value: &Option<Located<i64>>,
    report: &mut Report,
) -> Option<Option<Duration>> {
    match value {
        Some(value) => i64_secs_to_duration(value, report).map(Some),
        None => Some(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_range_value_narrows() {
        let mut report = Report::new();
        let value = Located::detached(8080_i64);

        assert_eq!(i64_to_u16(&value, &mut report), Some(8080_u16));
        assert!(!report.has_errors());
    }

    #[test]
    fn out_of_range_value_reports_and_fails() {
        let mut report = Report::new();
        let value = Located::detached(70_000_i64);

        assert_eq!(i64_to_u16(&value, &mut report), None);
        assert!(report.has_errors());
        assert_eq!(
            report.issues()[0].message,
            "value 70000 is out of range for u16"
        );
    }

    #[test]
    fn negative_value_reports_and_fails() {
        let mut report = Report::new();
        let value = Located::detached(-1_i64);

        assert_eq!(i64_to_u64(&value, &mut report), None);
        assert!(report.has_errors());
    }

    #[test]
    fn out_of_range_error_carries_the_span() {
        let mut sources = crate::source::SourceMap::new();
        let id = sources.add("test.hcl", "port = 99999");
        let span = crate::source::Span {
            source: id,
            start: 7,
            end: 12,
        };
        let mut report = Report::new();
        let value = Located {
            value: 99999_i64,
            span,
        };

        assert_eq!(i64_to_u16(&value, &mut report), None);
        assert_eq!(report.issues()[0].span, Some(span));
    }

    #[test]
    fn optional_absent_is_not_a_failure() {
        let mut report = Report::new();

        assert_eq!(opt_i64_to_usize(&None, &mut report), Some(None));
        assert!(!report.has_errors());
    }

    #[test]
    fn optional_present_narrows() {
        let mut report = Report::new();
        let value = Some(Located::detached(42_i64));

        assert_eq!(opt_i64_to_u32(&value, &mut report), Some(Some(42_u32)));
    }

    #[test]
    fn optional_out_of_range_fails() {
        let mut report = Report::new();
        let value = Some(Located::detached(-5_i64));

        assert_eq!(opt_i64_to_u32(&value, &mut report), None);
        assert!(report.has_errors());
    }

    #[test]
    fn seconds_convert_to_duration() {
        let mut report = Report::new();
        let value = Located::detached(60_i64);

        assert_eq!(
            i64_secs_to_duration(&value, &mut report),
            Some(Duration::from_secs(60))
        );
        assert!(!report.has_errors());
    }

    #[test]
    fn negative_seconds_report_rather_than_wrap() {
        let mut report = Report::new();
        let value = Located::detached(-1_i64);

        assert_eq!(i64_secs_to_duration(&value, &mut report), None);
        assert!(report.has_errors());
    }

    #[test]
    fn optional_absent_seconds_is_not_a_failure() {
        let mut report = Report::new();

        assert_eq!(opt_i64_secs_to_duration(&None, &mut report), Some(None));
        assert!(!report.has_errors());
    }

    #[test]
    fn optional_present_seconds_convert() {
        let mut report = Report::new();
        let value = Some(Located::detached(30_i64));

        assert_eq!(
            opt_i64_secs_to_duration(&value, &mut report),
            Some(Some(Duration::from_secs(30)))
        );
    }

    #[test]
    fn optional_negative_seconds_fail() {
        let mut report = Report::new();
        let value = Some(Located::detached(-5_i64));

        assert_eq!(opt_i64_secs_to_duration(&value, &mut report), None);
        assert!(report.has_errors());
    }

    #[test]
    fn i64_widens_to_f64() {
        let mut report = Report::new();
        let value = Located::detached(-42_i64);

        assert_eq!(i64_to_f64(&value, &mut report), Some(-42.0));
        assert!(!report.has_errors());
    }
}
