use crate::issue::ValidationIssue;
use crate::origin::Origin;
use crate::severity::Severity;
use std::fmt;

#[derive(Debug, Default)]
pub struct ValidationReport<O: Origin> {
    errors: Vec<ValidationIssue<O>>,
    warnings: Vec<ValidationIssue<O>>,
}

impl<O: Origin> ValidationReport<O> {
    /// Collect error
    pub fn error(&mut self, issue: ValidationIssue<O>) {
        self.errors.push(issue);
    }

    /// Collect warning
    pub fn warning(&mut self, issue: ValidationIssue<O>) {
        self.warnings.push(issue);
    }

    /// Collect and auto-route by severity.
    pub fn push(&mut self, issue: ValidationIssue<O>) {
        match issue.severity {
            Severity::Error => self.error(issue),
            Severity::Warning => self.warning(issue),
        }
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    pub fn has_issues(&self) -> bool {
        self.has_errors() || self.has_warnings()
    }

    pub fn errors(&self) -> &[ValidationIssue<O>] {
        &self.errors
    }

    pub fn warnings(&self) -> &[ValidationIssue<O>] {
        &self.warnings
    }

    /// Iterate over errors then warnings.
    pub fn iter(&self) -> impl Iterator<Item = &ValidationIssue<O>> {
        self.errors.iter().chain(self.warnings.iter())
    }

    /// Merge two reports into one.
    pub fn merge(&mut self, other: ValidationReport<O>) {
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
    }
}

impl<O: Origin> ValidationReport<O> {
    /// Compact, one-line-per-issue format for CI/scripts.
    pub fn render_plain(&self, w: &mut impl fmt::Write) -> fmt::Result {
        for issue in self.iter() {
            writeln!(
                w,
                "{}:{}: {}",
                issue.origin.source(),
                severity_label(&issue.severity),
                issue.message
            )?;
            if let Some(help) = &issue.help {
                writeln!(w, "  help: {}", help)?;
            }
        }
        Ok(())
    }

    /// Colorized, grouped-by-source format for terminals.
    #[cfg(feature = "color")]
    pub fn render_pretty(&self, w: &mut impl fmt::Write) -> fmt::Result {
        use owo_colors::OwoColorize;
        use std::collections::BTreeMap;

        if !self.has_issues() {
            return Ok(());
        }

        writeln!(
            w,
            "validation failed ({} errors, {} warnings)\n",
            self.errors.len(),
            self.warnings.len()
        )?;

        let mut by_source: BTreeMap<&str, Vec<&ValidationIssue<O>>> = BTreeMap::new();
        for issue in self.iter() {
            by_source
                .entry(issue.origin.source())
                .or_default()
                .push(issue);
        }

        for (source, issues) in by_source {
            writeln!(w, "{}", source)?;

            for issue in issues {
                let help = issue
                    .help
                    .as_ref()
                    .map(|h| format!("\n   help: {}", h))
                    .unwrap_or_default();

                match issue.severity {
                    Severity::Error => {
                        writeln!(w, "  {}: {}{}", "error".red().bold(), issue.message, help)?;
                    }
                    Severity::Warning => {
                        writeln!(
                            w,
                            "  {}: {}{}",
                            "warning".yellow().bold(),
                            issue.message,
                            help
                        )?;
                    }
                }
            }

            writeln!(w)?;
        }

        Ok(())
    }
}

#[cfg(feature = "serde")]
impl<O: Origin + serde::Serialize> ValidationReport<O> {
    pub fn render_json(&self, w: &mut impl std::io::Write) -> serde_json::Result<()> {
        #[derive(serde::Serialize)]
        struct IssueJson<'a, O: Origin + serde::Serialize> {
            severity: &'a str,
            message: &'a str,
            origin: &'a O,
            #[serde(skip_serializing_if = "Option::is_none")]
            help: &'a Option<String>,
        }

        #[derive(serde::Serialize)]
        struct ReportJson<'a, O: Origin + serde::Serialize> {
            errors: Vec<IssueJson<'a, O>>,
            warnings: Vec<IssueJson<'a, O>>,
        }

        fn to_json<'a, O: Origin + serde::Serialize>(
            issue: &'a ValidationIssue<O>,
        ) -> IssueJson<'a, O> {
            IssueJson {
                severity: severity_label(&issue.severity),
                message: &issue.message,
                origin: &issue.origin,
                help: &issue.help,
            }
        }

        let report = ReportJson {
            errors: self.errors().iter().map(to_json).collect(),
            warnings: self.warnings().iter().map(to_json).collect(),
        };

        serde_json::to_writer_pretty(w, &report)
    }
}

fn severity_label(severity: &Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SimpleOrigin;

    fn test_origin() -> SimpleOrigin {
        SimpleOrigin::new("test.toml", "server block")
    }

    #[test]
    fn empty_report_has_no_issues() {
        let report = ValidationReport::<SimpleOrigin>::default();
        assert!(!report.has_issues());
        assert!(!report.has_errors());
        assert!(!report.has_warnings());
    }

    #[test]
    fn error_tracked() {
        let mut report = ValidationReport::default();
        report.error(ValidationIssue::error("bad port", test_origin()));
        assert!(report.has_errors());
        assert!(!report.has_warnings());
        assert!(report.has_issues());
        assert_eq!(report.errors().len(), 1);
    }

    #[test]
    fn warning_tracked() {
        let mut report = ValidationReport::default();
        report.warning(ValidationIssue::warning("unused field", test_origin()));
        assert!(!report.has_errors());
        assert!(report.has_warnings());
        assert!(report.has_issues());
        assert_eq!(report.warnings().len(), 1);
    }

    #[test]
    fn push_routes_by_severity() {
        let mut report = ValidationReport::default();
        report.push(ValidationIssue::error("an error", test_origin()));
        report.push(ValidationIssue::warning("a warning", test_origin()));
        assert_eq!(report.errors().len(), 1);
        assert_eq!(report.warnings().len(), 1);
    }

    #[test]
    fn iter_yields_errors_then_warnings() {
        let mut report = ValidationReport::default();
        report.push(ValidationIssue::error("err", test_origin()));
        report.push(ValidationIssue::warning("warn", test_origin()));
        let messages: Vec<_> = report.iter().map(|i| i.message.as_str()).collect();
        assert_eq!(messages, vec!["err", "warn"]);
    }

    #[test]
    fn merge_combines_reports() {
        let mut a = ValidationReport::default();
        a.push(ValidationIssue::error("e1", test_origin()));

        let mut b = ValidationReport::default();
        b.push(ValidationIssue::error("e2", test_origin()));
        b.push(ValidationIssue::warning("w1", test_origin()));

        a.merge(b);
        assert_eq!(a.errors().len(), 2);
        assert_eq!(a.warnings().len(), 1);
    }

    #[test]
    fn render_plain_format() {
        let mut report = ValidationReport::default();
        report.push(ValidationIssue::error("bad port", test_origin()));
        report.push(ValidationIssue::error_with_help(
            "hostname empty",
            test_origin(),
            "set a hostname",
        ));

        let mut out = String::new();
        report.render_plain(&mut out).unwrap();

        assert!(out.contains("test.toml:error: bad port"));
        assert!(out.contains("test.toml:error: hostname empty"));
        assert!(out.contains("  help: set a hostname"));
    }

    #[test]
    fn render_plain_empty_report() {
        let report = ValidationReport::<SimpleOrigin>::default();
        let mut out = String::new();
        report.render_plain(&mut out).unwrap();
        assert!(out.is_empty());
    }
}
