//! Rendering of a [`Report`] into human- and machine-readable output.
//!
//! The report data model lives in [`report`](super::report); this module only
//! reads a finished report and formats it against a [`SourceMap`], resolving
//! each span's byte offset into a line and column at render time. Three formats
//! are offered: a compact one-line-per-issue [`render_plain`](Report::render_plain),
//! a colorized rustc-style [`render_pretty`](Report::render_pretty) (feature
//! `color`), and structured [`render_json`](Report::render_json) (feature
//! `serde`).

use crate::diagnostic::Severity;
use crate::diagnostic::report::Report;
use crate::source::{Source, SourceMap, Span};
use std::fmt;

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
        for issue in self.issues() {
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
            let issue = &self.issues()[index];
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
        let issues = self.issues();
        let mut group_of_source = Vec::new();
        let mut keyed: Vec<(usize, usize)> = Vec::with_capacity(issues.len());
        for (index, issue) in issues.iter().enumerate() {
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
    // their first line only. Snap to char boundaries so a span offset that
    // landed inside a multi-byte character cannot panic the slice below.
    let underline_start =
        source.floor_char_boundary((span.start as usize).clamp(line_start, line_end));
    let underline_end =
        source.ceil_char_boundary((span.end as usize).clamp(underline_start, line_end));
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
            .issues()
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
    use crate::diagnostic::Report;
    use crate::source::{SourceId, SourceMap, Span};

    fn one_source() -> (SourceMap, SourceId) {
        let mut sources = SourceMap::new();
        let id = sources.add("test.hcl", "port = 99999\nname = \"api\"\n");
        (sources, id)
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
    fn render_pretty_span_inside_multibyte_char_does_not_panic() {
        // "é" is two bytes; an offset..offset+1 span ending mid-character must
        // not panic the underline slice.
        let mut sources = SourceMap::new();
        let id = sources.add("test.hcl", "x = é\n");
        let bad_end = "x = é".find('é').unwrap() as u32 + 1;
        assert!(
            !sources
                .get(id)
                .unwrap()
                .text
                .is_char_boundary(bad_end as usize)
        );
        let mut report = Report::new();
        report
            .error("syntax error")
            .at(Span::new(id, bad_end - 1, bad_end))
            .emit();

        let mut out = String::new();
        report.render_pretty(&sources, &mut out).unwrap();
        assert!(out.contains("syntax error"), "got: {out}");
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
