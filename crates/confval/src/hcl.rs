//! HCL format adapter: produces trees of [`Located`] values from HCL text,
//! accumulating every problem into a [`Report`] instead of failing on the
//! first one.
//!
//! Behavior contract:
//!
//! - Syntax errors are pushed to the report with the parser's location and
//!   parsing returns `None`.
//! - Unknown fields are errors with the field's span; the walk continues so
//!   every unknown field in the file is reported.
//! - Missing required fields are errors with the enclosing block's span,
//!   reported after the whole body has been walked.
//! - Type mismatches are errors with the value's span; the field is then
//!   treated as missing, without an additional missing-field error.
//! - A `from_hcl` that returns `None` has always pushed at least one error.
//!
//! The leaf helpers are public so that hand-written and generated
//! [`FromHcl`] impls go through the same functions and report identically.

use crate::provenance::{Located, Report, SourceId, SourceMap, Span};
use hcl_edit::expr::Expression;
use hcl_edit::structure::{Attribute, Block, Body};

/// Structural construction of `Self` from an HCL body.
///
/// Implementations walk the body once, match attributes and blocks by name,
/// and push every problem they find to the report. Returning `None` means
/// at least one error was pushed.
pub trait FromHcl: Sized {
    fn from_hcl(body: &Body, source: SourceId, report: &mut Report) -> Option<Self>;
}

/// Parses one registered source into a `T`, pushing syntax errors and
/// structural problems into the report.
pub fn parse_hcl<T: FromHcl>(sources: &SourceMap, id: SourceId, report: &mut Report) -> Option<T> {
    let Some(source) = sources.get(id) else {
        report
            .error("internal error: parse_hcl called with an unregistered source id")
            .emit();
        return None;
    };
    match hcl_edit::parser::parse_body(&source.text) {
        Ok(body) => T::from_hcl(&body, id, report),
        Err(error) => {
            let offset = error.location().offset() as u32;
            report
                .error(format!("syntax error: {}", error.message()))
                .at(Span::new(id, offset, offset.saturating_add(1)))
                .emit();
            None
        }
    }
}

/// Converts an hcl-edit node's span to a confval [`Span`]. Nodes not
/// emitted by the parser have no span and map to a detached one.
pub fn span_of(node: &impl hcl_edit::Span, source: SourceId) -> Span {
    match node.span() {
        Some(range) => Span::new(source, range.start as u32, range.end as u32),
        None => Span::detached(),
    }
}

/// Parses a string attribute value. Reports a type mismatch and returns
/// `None` for anything that is not a literal string.
pub fn parse_string_attr(
    attr: &Attribute,
    source: SourceId,
    report: &mut Report,
) -> Option<Located<String>> {
    match attr.value.as_str() {
        Some(value) => Some(Located::new(
            value.to_string(),
            span_of(&attr.value, source),
        )),
        None => {
            report_type_mismatch(attr, "string", source, report);
            None
        }
    }
}

/// Parses an integer attribute value. HCL's native integer is `i64`;
/// narrowing happens at lowering, never here.
pub fn parse_int_attr(
    attr: &Attribute,
    source: SourceId,
    report: &mut Report,
) -> Option<Located<i64>> {
    match attr.value.as_number().and_then(|number| number.as_i64()) {
        Some(value) => Some(Located::new(value, span_of(&attr.value, source))),
        None => {
            report_type_mismatch(attr, "integer", source, report);
            None
        }
    }
}

/// Parses a boolean attribute value.
pub fn parse_bool_attr(
    attr: &Attribute,
    source: SourceId,
    report: &mut Report,
) -> Option<Located<bool>> {
    match attr.value.as_bool() {
        Some(value) => Some(Located::new(value, span_of(&attr.value, source))),
        None => {
            report_type_mismatch(attr, "bool", source, report);
            None
        }
    }
}

/// Parses an array-of-strings attribute with per-element spans, so an
/// invalid element is reported at that element, not at the whole list.
/// Every invalid element is reported; if any element is invalid the field
/// is treated as missing.
pub fn parse_string_list_attr(
    attr: &Attribute,
    source: SourceId,
    report: &mut Report,
) -> Option<Located<Vec<Located<String>>>> {
    let Some(array) = attr.value.as_array() else {
        report_type_mismatch(attr, "array of strings", source, report);
        return None;
    };
    let mut elements = Vec::new();
    let mut all_valid = true;
    for element in array.iter() {
        match element.as_str() {
            Some(value) => elements.push(Located::new(value.to_string(), span_of(element, source))),
            None => {
                report
                    .error(format!("expected string, found {}", describe(element)))
                    .at(span_of(element, source))
                    .emit();
                all_valid = false;
            }
        }
    }
    all_valid.then(|| Located::new(elements, span_of(&attr.value, source)))
}

/// Parses a nested block via the inner type's [`FromHcl`] impl. The
/// returned `Located` carries the block's own span, header through closing
/// brace.
pub fn parse_block<S: FromHcl>(
    block: &Block,
    source: SourceId,
    report: &mut Report,
) -> Option<Located<S>> {
    let span = span_of(block, source);
    S::from_hcl(&block.body, source, report).map(|spec| Located::new(spec, span))
}

/// Reports an unrecognized attribute at the attribute key's span.
pub fn report_unknown_field(attr: &Attribute, source: SourceId, report: &mut Report) {
    report
        .error(format!("unknown field: {}", attr.key.value().as_str()))
        .at(span_of(&attr.key, source))
        .emit();
}

/// Reports an unrecognized block at the block identifier's span.
pub fn report_unknown_block(block: &Block, source: SourceId, report: &mut Report) {
    report
        .error(format!("unknown block: {}", block.ident.value().as_str()))
        .at(span_of(&block.ident, source))
        .emit();
}

/// Reports a missing required field at the enclosing block's span.
pub fn report_missing_field(name: &str, enclosing: Span, report: &mut Report) {
    report
        .error(format!("missing required field: {name}"))
        .at(enclosing)
        .emit();
}

/// Reports a second occurrence of a single-occurrence field, pointing back
/// at the first. The first occurrence wins so parsing can continue.
pub fn report_duplicate_field(name: &str, second: Span, first: Span, report: &mut Report) {
    report
        .error(format!("duplicate field: {name}"))
        .at(second)
        .related(first, "first declared here")
        .emit();
}

/// Reports a value of the wrong type at the value's span.
pub fn report_type_mismatch(
    attr: &Attribute,
    expected: &str,
    source: SourceId,
    report: &mut Report,
) {
    report
        .error(format!(
            "expected {expected}, found {}",
            describe(&attr.value)
        ))
        .at(span_of(&attr.value, source))
        .emit();
}

fn describe(expr: &Expression) -> &'static str {
    match expr {
        Expression::Null(_) => "null",
        Expression::Bool(_) => "bool",
        Expression::Number(_) => "number",
        Expression::String(_) => "string",
        Expression::Array(_) => "array",
        Expression::Object(_) => "object",
        Expression::StringTemplate(_) | Expression::HeredocTemplate(_) => "string template",
        _ => "expression",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_attr(input: &str) -> (SourceMap, SourceId, Body) {
        let mut sources = SourceMap::new();
        let id = sources.add("test.hcl", input);
        let body = hcl_edit::parser::parse_body(&sources.get(id).unwrap().text).unwrap();
        (sources, id, body)
    }

    #[test]
    fn string_attr_parses_with_value_span() {
        let (_, id, body) = parse_attr("name = \"api\"\n");
        let attr = body.get_attribute("name").unwrap();
        let mut report = Report::new();
        let value = parse_string_attr(attr, id, &mut report).unwrap();
        assert_eq!(value.value, "api");
        assert_eq!(value.span, Span::new(id, 7, 12));
        assert!(!report.has_issues());
    }

    #[test]
    fn string_attr_type_mismatch_reports_and_returns_none() {
        let (_, id, body) = parse_attr("name = 42\n");
        let attr = body.get_attribute("name").unwrap();
        let mut report = Report::new();
        assert!(parse_string_attr(attr, id, &mut report).is_none());
        assert!(report.has_errors());
        assert_eq!(report.issues()[0].message, "expected string, found number");
        assert_eq!(report.issues()[0].span, Some(Span::new(id, 7, 9)));
    }

    #[test]
    fn int_attr_parses() {
        let (_, id, body) = parse_attr("port = 8080\n");
        let attr = body.get_attribute("port").unwrap();
        let mut report = Report::new();
        let value = parse_int_attr(attr, id, &mut report).unwrap();
        assert_eq!(value.value, 8080);
        assert!(!report.has_issues());
    }

    #[test]
    fn int_attr_rejects_floats() {
        let (_, id, body) = parse_attr("port = 1.5\n");
        let attr = body.get_attribute("port").unwrap();
        let mut report = Report::new();
        assert!(parse_int_attr(attr, id, &mut report).is_none());
        assert_eq!(report.issues()[0].message, "expected integer, found number");
    }

    #[test]
    fn bool_attr_parses() {
        let (_, id, body) = parse_attr("daemon = true\n");
        let attr = body.get_attribute("daemon").unwrap();
        let mut report = Report::new();
        let value = parse_bool_attr(attr, id, &mut report).unwrap();
        assert!(value.value);
    }

    #[test]
    fn string_list_attr_has_per_element_spans() {
        let input = "allow = [\"10.0.0.0/8\", \"192.168.0.0/16\"]\n";
        let (_, id, body) = parse_attr(input);
        let attr = body.get_attribute("allow").unwrap();
        let mut report = Report::new();
        let list = parse_string_list_attr(attr, id, &mut report).unwrap();
        assert_eq!(list.value.len(), 2);
        let first = &list.value[0];
        assert_eq!(first.value, "10.0.0.0/8");
        assert_eq!(
            &input[first.span.start as usize..first.span.end as usize],
            "\"10.0.0.0/8\""
        );
    }

    #[test]
    fn string_list_attr_reports_each_bad_element() {
        let (_, id, body) = parse_attr("allow = [\"ok\", 1, true]\n");
        let attr = body.get_attribute("allow").unwrap();
        let mut report = Report::new();
        assert!(parse_string_list_attr(attr, id, &mut report).is_none());
        assert_eq!(report.issues().len(), 2);
        assert_eq!(report.issues()[0].message, "expected string, found number");
        assert_eq!(report.issues()[1].message, "expected string, found bool");
    }

    #[test]
    fn string_list_attr_rejects_non_arrays() {
        let (_, id, body) = parse_attr("allow = \"10.0.0.0/8\"\n");
        let attr = body.get_attribute("allow").unwrap();
        let mut report = Report::new();
        assert!(parse_string_list_attr(attr, id, &mut report).is_none());
        assert_eq!(
            report.issues()[0].message,
            "expected array of strings, found string"
        );
    }

    #[test]
    fn syntax_error_is_reported_with_location() {
        let mut sources = SourceMap::new();
        let id = sources.add("broken.hcl", "server {\n  port =\n");
        let mut report = Report::new();

        struct Empty;
        impl FromHcl for Empty {
            fn from_hcl(_: &Body, _: SourceId, _: &mut Report) -> Option<Self> {
                Some(Empty)
            }
        }

        let parsed: Option<Empty> = parse_hcl(&sources, id, &mut report);
        assert!(parsed.is_none());
        assert!(report.has_errors());
        assert!(report.issues()[0].message.starts_with("syntax error:"));
        assert!(report.issues()[0].span.is_some());
    }

    #[test]
    fn unknown_field_reported_at_key_span() {
        let (_, id, body) = parse_attr("hostnme = \"typo\"\n");
        let attr = body.get_attribute("hostnme").unwrap();
        let mut report = Report::new();
        report_unknown_field(attr, id, &mut report);
        assert_eq!(report.issues()[0].message, "unknown field: hostnme");
        assert_eq!(report.issues()[0].span, Some(Span::new(id, 0, 7)));
    }

    #[test]
    fn duplicate_field_links_first_occurrence() {
        let mut report = Report::new();
        let first = Span::new(SourceId(0), 0, 4);
        let second = Span::new(SourceId(0), 20, 24);
        report_duplicate_field("port", second, first, &mut report);
        let issue = &report.issues()[0];
        assert_eq!(issue.message, "duplicate field: port");
        assert_eq!(issue.span, Some(second));
        assert_eq!(issue.related[0], (first, "first declared here".to_string()));
    }
}
