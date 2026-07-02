use crate::diagnostic::Severity;
use crate::source::Span;

/// A single finding: severity, message, optional primary span, optional
/// help text, and zero or more related spans (which may point into other
/// sources than the primary span).
#[derive(Debug, Clone)]
pub struct Issue {
    pub severity: Severity,
    pub message: String,
    pub span: Option<Span>,
    pub help: Option<String>,
    pub related: Vec<(Span, String)>,
}

/// Accumulates issues across parse, validation, and lowering.
///
/// The report is source-free: spans carry their [`SourceId`](crate::source::SourceId),
/// so reports from different files merge trivially and the
/// [`SourceMap`](crate::source::SourceMap) is only needed at render time.
#[derive(Debug, Default)]
pub struct Report {
    issues: Vec<Issue>,
}

impl Report {
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts building an error. The issue is recorded when the returned
    /// builder's [`emit`](IssueBuilder::emit) is called.
    pub fn error(&mut self, message: impl Into<String>) -> IssueBuilder<'_> {
        IssueBuilder::new(self, Severity::Error, message.into())
    }

    /// Starts building a warning. The issue is recorded when the returned
    /// builder's [`emit`](IssueBuilder::emit) is called.
    pub fn warning(&mut self, message: impl Into<String>) -> IssueBuilder<'_> {
        IssueBuilder::new(self, Severity::Warning, message.into())
    }

    pub fn has_errors(&self) -> bool {
        self.issues
            .iter()
            .any(|issue| issue.severity == Severity::Error)
    }

    pub fn has_warnings(&self) -> bool {
        self.issues
            .iter()
            .any(|issue| issue.severity == Severity::Warning)
    }

    pub fn has_issues(&self) -> bool {
        !self.issues.is_empty()
    }

    /// All issues in insertion order.
    pub fn issues(&self) -> &[Issue] {
        &self.issues
    }

    pub fn merge(&mut self, other: Report) {
        self.issues.extend(other.issues);
    }
}

/// Fluent construction of one [`Issue`]. Obtained from [`Report::error`] or
/// [`Report::warning`]; nothing is recorded until [`emit`](Self::emit).
#[must_use = "an issue is not recorded until .emit() is called"]
pub struct IssueBuilder<'a> {
    report: &'a mut Report,
    issue: Issue,
}

impl<'a> IssueBuilder<'a> {
    fn new(report: &'a mut Report, severity: Severity, message: String) -> Self {
        Self {
            report,
            issue: Issue {
                severity,
                message,
                span: None,
                help: None,
                related: Vec::new(),
            },
        }
    }

    /// Attributes the issue to a span. A detached span is treated as no
    /// location.
    pub fn at(mut self, span: Span) -> Self {
        self.issue.span = (!span.is_detached()).then_some(span);
        self
    }

    pub fn help(mut self, help: impl Into<String>) -> Self {
        self.issue.help = Some(help.into());
        self
    }

    /// Adds a secondary location, possibly in a different source than the
    /// primary span. Detached spans are ignored.
    pub fn related(mut self, span: Span, label: impl Into<String>) -> Self {
        if !span.is_detached() {
            self.issue.related.push((span, label.into()));
        }
        self
    }

    pub fn emit(self) {
        self.report.issues.push(self.issue);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{SourceMap, Span};

    fn one_source() -> (SourceMap, crate::source::SourceId) {
        let mut sources = SourceMap::new();
        let id = sources.add("test.hcl", "port = 99999\nname = \"api\"\n");
        (sources, id)
    }

    #[test]
    fn empty_report_has_no_issues() {
        let report = Report::new();
        assert!(!report.has_issues());
        assert!(!report.has_errors());
        assert!(!report.has_warnings());
    }

    #[test]
    fn builder_records_on_emit() {
        let mut report = Report::new();
        report.error("bad port").emit();
        assert!(report.has_errors());
        assert!(!report.has_warnings());
        assert_eq!(report.issues().len(), 1);
    }

    #[test]
    fn builder_captures_span_help_and_related() {
        let (_, id) = one_source();
        let mut report = Report::new();
        report
            .error("bad port")
            .at(Span::new(id, 7, 12))
            .help("use 1-65535")
            .related(Span::new(id, 0, 4), "declared here")
            .emit();

        let issue = &report.issues()[0];
        assert_eq!(issue.span, Some(Span::new(id, 7, 12)));
        assert_eq!(issue.help.as_deref(), Some("use 1-65535"));
        assert_eq!(issue.related.len(), 1);
    }

    #[test]
    fn detached_span_is_treated_as_no_location() {
        let mut report = Report::new();
        report.error("general problem").at(Span::detached()).emit();
        assert_eq!(report.issues()[0].span, None);
    }

    #[test]
    fn warnings_do_not_count_as_errors() {
        let mut report = Report::new();
        report.warning("deprecated").emit();
        assert!(!report.has_errors());
        assert!(report.has_warnings());
    }

    #[test]
    fn merge_combines_issues() {
        let mut a = Report::new();
        a.error("e1").emit();
        let mut b = Report::new();
        b.warning("w1").emit();
        a.merge(b);
        assert_eq!(a.issues().len(), 2);
        assert!(a.has_errors());
        assert!(a.has_warnings());
    }
}
