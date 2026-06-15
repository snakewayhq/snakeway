use crate::diagnostic::Severity;
use crate::source::{Source, SourceMap, Span};
use std::fmt;

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
/// The report is source-free: spans carry their [`SourceId`](crate::provenance::SourceId),
/// so reports from different files merge trivially and the
/// [`SourceMap`] is only needed at render time.
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

fn severity_label(severity: &Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
    }
}

fn resolve(sources: &SourceMap, span: Span) -> Option<(&Source, usize, usize)> {
    let source = sources.get(span.source)?;
    let (line, column) = source.line_column(span.start);
    Some((source, line, column))
}

impl Report {
    /// Compact, one-line-per-issue format for CI/scripts.
    pub fn render_plain(&self, sources: &SourceMap, w: &mut impl fmt::Write) -> fmt::Result {
        for issue in &self.issues {
            let severity = severity_label(&issue.severity);
            match issue.span.and_then(|span| resolve(sources, span)) {
                Some((source, line, column)) => writeln!(
                    w,
                    "{}:{}:{}: {}: {}",
                    source.name, line, column, severity, issue.message
                )?,
                None => writeln!(w, "{}: {}", severity, issue.message)?,
            }
            if let Some(help) = &issue.help {
                writeln!(w, "  help: {}", help)?;
            }
            for (span, label) in &issue.related {
                if let Some((source, line, column)) = resolve(sources, *span) {
                    writeln!(
                        w,
                        "  related: {}:{}:{}: {}",
                        source.name, line, column, label
                    )?;
                }
            }
        }
        Ok(())
    }

    /// Colorized, rustc-style format: severity header, location line, source
    /// excerpt with a caret underline, help text, and related locations.
    /// Issues are grouped by source, sources ordered by first appearance;
    /// issues without a location come last.
    #[cfg(feature = "color")]
    pub fn render_pretty(&self, sources: &SourceMap, w: &mut impl fmt::Write) -> fmt::Result {
        use owo_colors::OwoColorize;

        let order = self.grouped_order();

        for index in order {
            let issue = &self.issues[index];
            let header = match issue.severity {
                Severity::Error => "error".red().bold().to_string(),
                Severity::Warning => "warning".yellow().bold().to_string(),
            };
            writeln!(w, "{}: {}", header, issue.message)?;

            // One gutter width per issue: the widest line number among the
            // primary and related excerpts.
            let mut excerpt_lines = Vec::new();
            if let Some((_, line, _)) = issue.span.and_then(|span| resolve(sources, span)) {
                excerpt_lines.push(line);
            }
            for (span, _) in &issue.related {
                if let Some((_, line, _)) = resolve(sources, *span) {
                    excerpt_lines.push(line);
                }
            }
            let width = excerpt_lines
                .iter()
                .map(|line| line.to_string().len())
                .max()
                .unwrap_or(1);
            let pad = " ".repeat(width);

            if let Some(span) = issue.span {
                let underline = match issue.severity {
                    Severity::Error => UnderlineStyle::ErrorCaret,
                    Severity::Warning => UnderlineStyle::WarningCaret,
                };
                write_excerpt(w, sources, span, None, underline, &pad)?;
            }
            if let Some(help) = &issue.help {
                writeln!(w, "{pad} {} help: {}", "=".dimmed(), help)?;
            }
            for (span, label) in &issue.related {
                write_excerpt(
                    w,
                    sources,
                    *span,
                    Some(label.as_str()),
                    UnderlineStyle::RelatedDash,
                    &pad,
                )?;
            }
            writeln!(w)?;
        }
        Ok(())
    }

    /// Issue indices, grouped by primary-span source in order of first
    /// appearance, location-less issues last, insertion order within groups.
    #[cfg(feature = "color")]
    fn grouped_order(&self) -> Vec<usize> {
        let mut group_of_source = Vec::new();
        let mut keyed: Vec<(usize, usize)> = Vec::with_capacity(self.issues.len());
        for (index, issue) in self.issues.iter().enumerate() {
            let key = match issue.span {
                Some(span) => {
                    let position = group_of_source
                        .iter()
                        .position(|known| *known == span.source);
                    match position {
                        Some(group) => group,
                        None => {
                            group_of_source.push(span.source);
                            group_of_source.len() - 1
                        }
                    }
                }
                None => usize::MAX,
            };
            keyed.push((key, index));
        }
        keyed.sort_by_key(|(key, _)| *key);
        keyed.into_iter().map(|(_, index)| index).collect()
    }
}

#[cfg(feature = "color")]
enum UnderlineStyle {
    ErrorCaret,
    WarningCaret,
    RelatedDash,
}

/// Writes one source excerpt:
/// ```text
///  --> ingress.d/api.hcl:3:11
///   |
/// 3 |   allow = ["10.0.0.0/8", "bad"]
///   |                          ^^^^^
/// ```
#[cfg(feature = "color")]
fn write_excerpt(
    w: &mut impl fmt::Write,
    sources: &SourceMap,
    span: Span,
    label: Option<&str>,
    style: UnderlineStyle,
    pad: &str,
) -> fmt::Result {
    use owo_colors::OwoColorize;

    let Some((source, line, column)) = resolve(sources, span) else {
        return Ok(());
    };
    let Some((line_start, line_end)) = source.line_byte_range(line) else {
        return Ok(());
    };
    let text = &source.text[line_start..line_end];

    match label {
        Some(label) => writeln!(
            w,
            "{pad}{} {}:{}:{} ({})",
            "-->".dimmed(),
            source.name,
            line,
            column,
            label
        )?,
        None => writeln!(
            w,
            "{pad}{} {}:{}:{}",
            "-->".dimmed(),
            source.name,
            line,
            column
        )?,
    }
    writeln!(w, "{pad} {}", "|".dimmed())?;
    writeln!(
        w,
        "{:>width$} {} {}",
        line,
        "|".dimmed(),
        text,
        width = pad.len()
    )?;

    // Clamp the underline to the excerpt's line; multi-line spans underline
    // their first line only.
    let underline_start = (span.start as usize).clamp(line_start, line_end);
    let underline_end = (span.end as usize).clamp(underline_start, line_end);
    let length = source.text[underline_start..underline_end]
        .chars()
        .count()
        .max(1);
    let indent = " ".repeat(column - 1);
    let underline = match style {
        UnderlineStyle::ErrorCaret => "^".repeat(length).red().bold().to_string(),
        UnderlineStyle::WarningCaret => "^".repeat(length).yellow().bold().to_string(),
        UnderlineStyle::RelatedDash => "-".repeat(length).blue().to_string(),
    };
    writeln!(w, "{pad} {} {}{}", "|".dimmed(), indent, underline)?;
    Ok(())
}

#[cfg(feature = "serde")]
impl Report {
    /// Structured JSON for tooling, with resolved line/column alongside raw
    /// byte offsets.
    pub fn render_json(&self, sources: &SourceMap, w: &mut impl fmt::Write) -> fmt::Result {
        #[derive(serde::Serialize)]
        struct LocationJson<'a> {
            source: &'a str,
            line: usize,
            column: usize,
            start: u32,
            end: u32,
        }

        #[derive(serde::Serialize)]
        struct RelatedJson<'a> {
            #[serde(skip_serializing_if = "Option::is_none")]
            location: Option<LocationJson<'a>>,
            label: &'a str,
        }

        #[derive(serde::Serialize)]
        struct IssueJson<'a> {
            severity: &'a str,
            message: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            location: Option<LocationJson<'a>>,
            #[serde(skip_serializing_if = "Option::is_none")]
            help: Option<&'a str>,
            #[serde(skip_serializing_if = "Vec::is_empty")]
            related: Vec<RelatedJson<'a>>,
        }

        #[derive(serde::Serialize)]
        struct ReportJson<'a> {
            issues: Vec<IssueJson<'a>>,
        }

        fn location_json<'a>(sources: &'a SourceMap, span: Span) -> Option<LocationJson<'a>> {
            let (source, line, column) = resolve(sources, span)?;
            Some(LocationJson {
                source: &source.name,
                line,
                column,
                start: span.start,
                end: span.end,
            })
        }

        let issues = self
            .issues
            .iter()
            .map(|issue| IssueJson {
                severity: severity_label(&issue.severity),
                message: &issue.message,
                location: issue.span.and_then(|span| location_json(sources, span)),
                help: issue.help.as_deref(),
                related: issue
                    .related
                    .iter()
                    .map(|(span, label)| RelatedJson {
                        location: location_json(sources, *span),
                        label,
                    })
                    .collect(),
            })
            .collect();

        let rendered =
            serde_json::to_string_pretty(&ReportJson { issues }).map_err(|_| fmt::Error)?;
        w.write_str(&rendered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one_source() -> (SourceMap, crate::provenance::SourceId) {
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

    #[test]
    fn render_plain_includes_location_and_help() {
        let (sources, id) = one_source();
        let mut report = Report::new();
        report
            .error("port out of range")
            .at(Span::new(id, 7, 12))
            .help("use 1-65535")
            .emit();

        let mut out = String::new();
        report.render_plain(&sources, &mut out).unwrap();
        assert!(
            out.contains("test.hcl:1:8: error: port out of range"),
            "got: {out}"
        );
        assert!(out.contains("  help: use 1-65535"), "got: {out}");
    }

    #[test]
    fn render_plain_without_location() {
        let sources = SourceMap::new();
        let mut report = Report::new();
        report.error("no ingress files found").emit();

        let mut out = String::new();
        report.render_plain(&sources, &mut out).unwrap();
        assert_eq!(out, "error: no ingress files found\n");
    }

    #[test]
    fn render_plain_includes_related_locations() {
        let mut sources = SourceMap::new();
        let a = sources.add("a.hcl", "bind = \"127.0.0.1:80\"\n");
        let b = sources.add("b.hcl", "bind = \"127.0.0.1:80\"\n");
        let mut report = Report::new();
        report
            .error("duplicate bind address")
            .at(Span::new(b, 0, 4))
            .related(Span::new(a, 0, 4), "first declared here")
            .emit();

        let mut out = String::new();
        report.render_plain(&sources, &mut out).unwrap();
        assert!(
            out.contains("b.hcl:1:1: error: duplicate bind address"),
            "got: {out}"
        );
        assert!(
            out.contains("  related: a.hcl:1:1: first declared here"),
            "got: {out}"
        );
    }

    #[cfg(feature = "color")]
    #[test]
    fn render_pretty_underlines_the_span() {
        let (sources, id) = one_source();
        let mut report = Report::new();
        // Span of "99999" on line 1, columns 8-12.
        report
            .error("port out of range")
            .at(Span::new(id, 7, 12))
            .emit();

        let mut out = String::new();
        report.render_pretty(&sources, &mut out).unwrap();
        assert!(out.contains("test.hcl:1:8"), "got: {out}");
        assert!(out.contains("port = 99999"), "got: {out}");
        assert!(out.contains("^^^^^"), "got: {out}");
    }

    #[cfg(feature = "color")]
    #[test]
    fn render_pretty_cross_file_related_span() {
        let mut sources = SourceMap::new();
        let a = sources.add("a.hcl", "bind = \"127.0.0.1:80\"\n");
        let b = sources.add("b.hcl", "bind = \"127.0.0.1:80\"\n");
        let mut report = Report::new();
        report
            .error("duplicate bind address")
            .at(Span::new(b, 7, 21))
            .related(Span::new(a, 7, 21), "first declared here")
            .emit();

        let mut out = String::new();
        report.render_pretty(&sources, &mut out).unwrap();
        assert!(out.contains("b.hcl:1:8"), "got: {out}");
        assert!(
            out.contains("a.hcl:1:8 (first declared here)"),
            "got: {out}"
        );
        assert!(out.contains("--------------"), "got: {out}");
    }

    #[cfg(feature = "color")]
    #[test]
    fn render_pretty_groups_by_source_first_appearance() {
        let mut sources = SourceMap::new();
        let a = sources.add("a.hcl", "x = 1\n");
        let b = sources.add("b.hcl", "y = 2\n");
        let mut report = Report::new();
        report.error("first in a").at(Span::new(a, 0, 1)).emit();
        report.error("only in b").at(Span::new(b, 0, 1)).emit();
        report.error("second in a").at(Span::new(a, 4, 5)).emit();
        report.error("no location").emit();

        let mut out = String::new();
        report.render_pretty(&sources, &mut out).unwrap();
        let first_a = out.find("first in a").unwrap();
        let second_a = out.find("second in a").unwrap();
        let only_b = out.find("only in b").unwrap();
        let unlocated = out.find("no location").unwrap();
        assert!(first_a < second_a, "a-issues stay adjacent: {out}");
        assert!(second_a < only_b, "a-group precedes b-group: {out}");
        assert!(only_b < unlocated, "location-less issues come last: {out}");
    }

    #[cfg(feature = "serde")]
    #[test]
    fn render_json_resolves_locations() {
        let (sources, id) = one_source();
        let mut report = Report::new();
        report
            .error("port out of range")
            .at(Span::new(id, 7, 12))
            .help("use 1-65535")
            .emit();

        let mut out = String::new();
        report.render_json(&sources, &mut out).unwrap();
        let value: serde_json::Value = serde_json::from_str(&out).unwrap();
        let issue = &value["issues"][0];
        assert_eq!(issue["severity"], "error");
        assert_eq!(issue["message"], "port out of range");
        assert_eq!(issue["location"]["source"], "test.hcl");
        assert_eq!(issue["location"]["line"], 1);
        assert_eq!(issue["location"]["column"], 8);
        assert_eq!(issue["location"]["start"], 7);
        assert_eq!(issue["help"], "use 1-65535");
    }
}
