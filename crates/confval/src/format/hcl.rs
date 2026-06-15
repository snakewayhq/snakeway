//! HCL format adapter: produces trees of [`Located`] values from HCL text,
//! accumulating every problem into a [`Report`] instead of failing on the
//! first one.
//!
//! HCL offers two spellings for nested structures: blocks (`server { ... }`)
//! and object-valued attributes (`server = { ... }`). Both are accepted and
//! produce identical Spec trees; [`Fields`] normalizes body structures and
//! object items into one view so implementations handle them uniformly.
//!
//! Behavior contract:
//!
//! - Syntax errors are pushed to the report with the parser's location and
//!   parsing returns `None`.
//! - Unknown fields are errors with the field name's span; the walk
//!   continues so every unknown field in the file is reported.
//! - Missing required fields are errors with the enclosing block's span,
//!   reported after the whole body has been walked.
//! - Type mismatches are errors with the value's span; the field is then
//!   treated as missing, without an additional missing-field error.
//! - A `from_hcl` that returns `None` has always pushed at least one error.
//!
//! The field parsers are public so that hand-written and generated
//! [`FromHcl`] impls go through the same functions and report identically.

use crate::diagnostic::Report;
use crate::source::{Located, SourceId, SourceMap, Span};
use hcl_edit::expr::{Expression, Object, ObjectKey};
use hcl_edit::structure::{Block, Body, Structure};

/// Structural construction of `Self` from a normalized field view.
///
/// Implementations walk the fields once, match them by name, and push every
/// problem they find to the report. Returning `None` means at least one
/// error was pushed.
pub trait FromHcl: Sized {
    fn from_hcl(fields: &Fields<'_>, report: &mut Report) -> Option<Self>;
}

/// One named field: an attribute, a block, or an object item.
#[derive(Debug)]
pub struct Field<'a> {
    pub name: &'a str,
    /// Span of the field's name (attribute key, block identifier, or
    /// object key).
    pub name_span: Span,
    /// Span of the whole field.
    pub span: Span,
    pub source: SourceId,
    pub kind: FieldKind<'a>,
}

#[derive(Debug)]
pub enum FieldKind<'a> {
    /// An attribute value or object item value.
    Value(&'a Expression),
    /// A block with its own body.
    Block(&'a Block),
}

/// The normalized fields of one structural level: a body's attributes and
/// blocks, or an object's items.
#[derive(Debug)]
pub struct Fields<'a> {
    source: SourceId,
    enclosing: Span,
    items: Vec<Field<'a>>,
}

impl<'a> Fields<'a> {
    /// Normalizes a body's attributes and blocks. `enclosing` is the span
    /// missing-field errors point at: the surrounding block, or the whole
    /// file at the root.
    pub fn of_body(body: &'a Body, enclosing: Span, source: SourceId) -> Self {
        let mut items = Vec::new();
        for structure in body.iter() {
            match structure {
                Structure::Attribute(attr) => items.push(Field {
                    name: attr.key.value().as_str(),
                    name_span: span_of(&attr.key, source),
                    span: span_of(attr, source),
                    source,
                    kind: FieldKind::Value(&attr.value),
                }),
                Structure::Block(block) => items.push(Field {
                    name: block.ident.value().as_str(),
                    name_span: span_of(&block.ident, source),
                    span: span_of(block, source),
                    source,
                    kind: FieldKind::Block(block),
                }),
            }
        }
        Self {
            source,
            enclosing,
            items,
        }
    }

    /// Normalizes an object's items. Non-identifier, non-string keys are
    /// reported and skipped.
    pub fn of_object(
        object: &'a Object,
        enclosing: Span,
        source: SourceId,
        report: &mut Report,
    ) -> Self {
        let mut items = Vec::new();
        for (key, value) in object.iter() {
            let name = match key {
                ObjectKey::Ident(ident) => ident.value().as_str(),
                ObjectKey::Expression(expr) => match expr.as_str() {
                    Some(name) => name,
                    None => {
                        report
                            .error("expected an identifier or string as object key")
                            .at(span_of(key, source))
                            .emit();
                        continue;
                    }
                },
            };
            let name_span = span_of(key, source);
            let value_span = span_of(value.expr(), source);
            items.push(Field {
                name,
                name_span,
                span: Span::merge(name_span, value_span),
                source,
                kind: FieldKind::Value(value.expr()),
            });
        }
        Self {
            source,
            enclosing,
            items,
        }
    }

    pub fn source(&self) -> SourceId {
        self.source
    }

    /// The span missing-field errors point at.
    pub fn enclosing(&self) -> Span {
        self.enclosing
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Field<'a>> {
        self.items.iter()
    }

    pub fn has(&self, name: &str) -> bool {
        self.items.iter().any(|field| field.name == name)
    }

    pub fn get(&self, name: &str) -> Option<&Field<'a>> {
        self.items.iter().find(|field| field.name == name)
    }
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
        Ok(body) => {
            let enclosing = Span::new(id, 0, source.text.len() as u32);
            let fields = Fields::of_body(&body, enclosing, id);
            T::from_hcl(&fields, report)
        }
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

fn expect_value<'a, 'f>(
    field: &'f Field<'a>,
    expected: &str,
    report: &mut Report,
) -> Option<&'f &'a Expression> {
    match &field.kind {
        FieldKind::Value(expr) => Some(expr),
        FieldKind::Block(_) => {
            report
                .error(format!("expected {expected}, found block"))
                .at(field.span)
                .emit();
            None
        }
    }
}

/// Parses a string field. Reports a type mismatch and returns `None` for
/// anything that is not a literal string.
pub fn parse_string_field(field: &Field<'_>, report: &mut Report) -> Option<Located<String>> {
    let expr = expect_value(field, "string", report)?;
    match expr.as_str() {
        Some(value) => Some(Located::new(
            value.to_string(),
            span_of(*expr, field.source),
        )),
        None => {
            report_type_mismatch(expr, "string", field.source, report);
            None
        }
    }
}

/// Parses an integer field. HCL's native integer is `i64`; narrowing
/// happens at lowering, never here.
pub fn parse_int_field(field: &Field<'_>, report: &mut Report) -> Option<Located<i64>> {
    let expr = expect_value(field, "integer", report)?;
    match expr.as_number().and_then(|number| number.as_i64()) {
        Some(value) => Some(Located::new(value, span_of(*expr, field.source))),
        None => {
            report_type_mismatch(expr, "integer", field.source, report);
            None
        }
    }
}

/// Parses a float field. Integer literals widen losslessly where possible,
/// matching HCL's single number type.
pub fn parse_float_field(field: &Field<'_>, report: &mut Report) -> Option<Located<f64>> {
    let expr = expect_value(field, "number", report)?;
    match expr.as_number().and_then(|number| number.as_f64()) {
        Some(value) => Some(Located::new(value, span_of(*expr, field.source))),
        None => {
            report_type_mismatch(expr, "number", field.source, report);
            None
        }
    }
}

/// Parses a boolean field.
pub fn parse_bool_field(field: &Field<'_>, report: &mut Report) -> Option<Located<bool>> {
    let expr = expect_value(field, "bool", report)?;
    match expr.as_bool() {
        Some(value) => Some(Located::new(value, span_of(*expr, field.source))),
        None => {
            report_type_mismatch(expr, "bool", field.source, report);
            None
        }
    }
}

/// Parses an array-of-strings field with per-element spans, so an invalid
/// element is reported at that element, not at the whole list. Every
/// invalid element is reported; if any element is invalid the field is
/// treated as missing.
pub fn parse_string_list_field(
    field: &Field<'_>,
    report: &mut Report,
) -> Option<Located<Vec<Located<String>>>> {
    let expr = expect_value(field, "array of strings", report)?;
    let Some(array) = expr.as_array() else {
        report_type_mismatch(expr, "array of strings", field.source, report);
        return None;
    };
    let mut elements = Vec::new();
    let mut all_valid = true;
    for element in array.iter() {
        match element.as_str() {
            Some(value) => elements.push(Located::new(
                value.to_string(),
                span_of(element, field.source),
            )),
            None => {
                report
                    .error(format!("expected string, found {}", describe(element)))
                    .at(span_of(element, field.source))
                    .emit();
                all_valid = false;
            }
        }
    }
    all_valid.then(|| Located::new(elements, span_of(*expr, field.source)))
}

/// Parses a nested structure via the inner type's [`FromHcl`] impl. Accepts
/// both spellings: a block, or an attribute whose value is an object. The
/// returned `Located` carries the whole structure's span.
pub fn parse_struct_field<S: FromHcl>(
    field: &Field<'_>,
    report: &mut Report,
) -> Option<Located<S>> {
    match &field.kind {
        FieldKind::Block(block) => {
            let enclosing = span_of(*block, field.source);
            let fields = Fields::of_body(&block.body, enclosing, field.source);
            S::from_hcl(&fields, report).map(|spec| Located::new(spec, enclosing))
        }
        FieldKind::Value(expr) => match expr {
            Expression::Object(object) => {
                let enclosing = span_of(*expr, field.source);
                let fields = Fields::of_object(object, enclosing, field.source, report);
                S::from_hcl(&fields, report).map(|spec| Located::new(spec, enclosing))
            }
            other => {
                report_type_mismatch(other, "block", field.source, report);
                None
            }
        },
    }
}

/// Parses a repeated nested structure into `slot`, appending. Accepts both
/// spellings and combinations of them: each repeated block appends one
/// element, and an array-of-objects attribute appends one element per
/// object. Invalid array elements are reported individually and skipped.
pub fn parse_struct_list_field<S: FromHcl>(
    slot: &mut Vec<Located<S>>,
    field: &Field<'_>,
    report: &mut Report,
) {
    match &field.kind {
        FieldKind::Block(_) => {
            if let Some(parsed) = parse_struct_field(field, report) {
                slot.push(parsed);
            }
        }
        FieldKind::Value(expr) => match expr {
            Expression::Array(array) => {
                for element in array.iter() {
                    match element {
                        Expression::Object(object) => {
                            let enclosing = span_of(element, field.source);
                            let fields = Fields::of_object(object, enclosing, field.source, report);
                            if let Some(parsed) = S::from_hcl(&fields, report) {
                                slot.push(Located::new(parsed, enclosing));
                            }
                        }
                        other => {
                            report_type_mismatch(other, "object", field.source, report);
                        }
                    }
                }
            }
            other => {
                report_type_mismatch(other, "block or array of objects", field.source, report);
            }
        },
    }
}

/// Parses a single-occurrence nested structure into `slot`, tracking the
/// first occurrence in `seen` so a repeated one is reported as a duplicate
/// pointing back at the first. The first occurrence wins.
pub fn parse_single_struct<S: FromHcl>(
    slot: &mut Option<Located<S>>,
    seen: &mut Option<Span>,
    name: &str,
    field: &Field<'_>,
    report: &mut Report,
) {
    if let Some(first) = *seen {
        report_duplicate_field(name, field.span, first, report);
    } else {
        *seen = Some(field.span);
        *slot = parse_struct_field(field, report);
    }
}

/// Reports an unrecognized field at its name's span.
pub fn report_unknown_field(field: &Field<'_>, report: &mut Report) {
    let label = match field.kind {
        FieldKind::Value(_) => "field",
        FieldKind::Block(_) => "block",
    };
    report
        .error(format!("unknown {label}: {}", field.name))
        .at(field.name_span)
        .emit();
}

/// Reports a missing required field at the enclosing structure's span.
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

fn report_type_mismatch(expr: &Expression, expected: &str, source: SourceId, report: &mut Report) {
    report
        .error(format!("expected {expected}, found {}", describe(expr)))
        .at(span_of(expr, source))
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

    struct Probe;
    impl FromHcl for Probe {
        fn from_hcl(_: &Fields<'_>, _: &mut Report) -> Option<Self> {
            Some(Probe)
        }
    }

    fn parse_fields(input: &str) -> (SourceMap, SourceId, Body) {
        let mut sources = SourceMap::new();
        let id = sources.add("test.hcl", input);
        let body = hcl_edit::parser::parse_body(&sources.get(id).unwrap().text).unwrap();
        (sources, id, body)
    }

    fn fields_of<'a>(body: &'a Body, id: SourceId) -> Fields<'a> {
        Fields::of_body(body, Span::new(id, 0, 0), id)
    }

    #[test]
    fn string_field_parses_with_value_span() {
        let (_, id, body) = parse_fields("name = \"api\"\n");
        let fields = fields_of(&body, id);
        let mut report = Report::new();
        let value = parse_string_field(fields.get("name").unwrap(), &mut report).unwrap();
        assert_eq!(value.value, "api");
        assert_eq!(value.span, Span::new(id, 7, 12));
        assert!(!report.has_issues());
    }

    #[test]
    fn string_field_type_mismatch_reports_and_returns_none() {
        let (_, id, body) = parse_fields("name = 42\n");
        let fields = fields_of(&body, id);
        let mut report = Report::new();
        assert!(parse_string_field(fields.get("name").unwrap(), &mut report).is_none());
        assert!(report.has_errors());
        assert_eq!(report.issues()[0].message, "expected string, found number");
        assert_eq!(report.issues()[0].span, Some(Span::new(id, 7, 9)));
    }

    #[test]
    fn int_field_parses() {
        let (_, id, body) = parse_fields("port = 8080\n");
        let fields = fields_of(&body, id);
        let mut report = Report::new();
        let value = parse_int_field(fields.get("port").unwrap(), &mut report).unwrap();
        assert_eq!(value.value, 8080);
        assert!(!report.has_issues());
    }

    #[test]
    fn int_field_rejects_floats() {
        let (_, id, body) = parse_fields("port = 1.5\n");
        let fields = fields_of(&body, id);
        let mut report = Report::new();
        assert!(parse_int_field(fields.get("port").unwrap(), &mut report).is_none());
        assert_eq!(report.issues()[0].message, "expected integer, found number");
    }

    #[test]
    fn float_field_parses_floats_and_integers() {
        let (_, id, body) = parse_fields("ratio = 0.5\nwhole = 1\n");
        let fields = fields_of(&body, id);
        let mut report = Report::new();
        let ratio = parse_float_field(fields.get("ratio").unwrap(), &mut report);
        let whole = parse_float_field(fields.get("whole").unwrap(), &mut report);
        assert_eq!(ratio.unwrap().value, 0.5);
        assert_eq!(whole.unwrap().value, 1.0);
        assert!(!report.has_issues());
    }

    #[test]
    fn bool_field_parses() {
        let (_, id, body) = parse_fields("daemon = true\n");
        let fields = fields_of(&body, id);
        let mut report = Report::new();
        let value = parse_bool_field(fields.get("daemon").unwrap(), &mut report).unwrap();
        assert!(value.value);
    }

    #[test]
    fn string_list_field_has_per_element_spans() {
        let input = "allow = [\"10.0.0.0/8\", \"192.168.0.0/16\"]\n";
        let (_, id, body) = parse_fields(input);
        let fields = fields_of(&body, id);
        let mut report = Report::new();
        let list = parse_string_list_field(fields.get("allow").unwrap(), &mut report).unwrap();
        assert_eq!(list.value.len(), 2);
        let first = &list.value[0];
        assert_eq!(first.value, "10.0.0.0/8");
        assert_eq!(
            &input[first.span.start as usize..first.span.end as usize],
            "\"10.0.0.0/8\""
        );
    }

    #[test]
    fn string_list_field_reports_each_bad_element() {
        let (_, id, body) = parse_fields("allow = [\"ok\", 1, true]\n");
        let fields = fields_of(&body, id);
        let mut report = Report::new();
        assert!(parse_string_list_field(fields.get("allow").unwrap(), &mut report).is_none());
        assert_eq!(report.issues().len(), 2);
        assert_eq!(report.issues()[0].message, "expected string, found number");
        assert_eq!(report.issues()[1].message, "expected string, found bool");
    }

    #[test]
    fn struct_field_accepts_block_form() {
        let input = "tls {\n  cert = \"a.pem\"\n}\n";
        let (_, id, body) = parse_fields(input);
        let fields = fields_of(&body, id);
        let mut report = Report::new();
        let parsed: Option<Located<Probe>> =
            parse_struct_field(fields.get("tls").unwrap(), &mut report);
        let span = parsed.unwrap().span;
        assert_eq!(
            &input[span.start as usize..span.end as usize],
            "tls {\n  cert = \"a.pem\"\n}"
        );
    }

    #[test]
    fn struct_field_accepts_object_form() {
        let input = "tls = {\n  cert = \"a.pem\"\n}\n";
        let (_, id, body) = parse_fields(input);
        let fields = fields_of(&body, id);
        let mut report = Report::new();
        let parsed: Option<Located<Probe>> =
            parse_struct_field(fields.get("tls").unwrap(), &mut report);
        assert!(parsed.is_some());
        assert!(!report.has_issues());
    }

    #[test]
    fn struct_field_rejects_scalars() {
        let (_, id, body) = parse_fields("tls = true\n");
        let fields = fields_of(&body, id);
        let mut report = Report::new();
        let parsed: Option<Located<Probe>> =
            parse_struct_field(fields.get("tls").unwrap(), &mut report);
        assert!(parsed.is_none());
        assert_eq!(report.issues()[0].message, "expected block, found bool");
    }

    #[test]
    fn object_items_have_name_and_value_spans() {
        let input = "tls = {\n  cert = \"a.pem\"\n}\n";
        let (_, id, body) = parse_fields(input);
        let fields = fields_of(&body, id);
        let mut report = Report::new();
        let FieldKind::Value(Expression::Object(object)) = &fields.get("tls").unwrap().kind else {
            panic!("expected object value");
        };
        let inner = Fields::of_object(object, Span::detached(), id, &mut report);
        let cert = inner.get("cert").unwrap();
        assert_eq!(
            &input[cert.name_span.start as usize..cert.name_span.end as usize],
            "cert"
        );
        let value = parse_string_field(cert, &mut report).unwrap();
        assert_eq!(
            &input[value.span.start as usize..value.span.end as usize],
            "\"a.pem\""
        );
    }

    #[test]
    fn struct_list_field_appends_repeated_blocks() {
        let input = "service {\n  a = 1\n}\nservice {\n  b = 2\n}\n";
        let (_, id, body) = parse_fields(input);
        let fields = fields_of(&body, id);
        let mut report = Report::new();
        let mut services: Vec<Located<Probe>> = Vec::new();
        for field in fields.iter() {
            parse_struct_list_field(&mut services, field, &mut report);
        }
        assert_eq!(services.len(), 2);
        assert!(!report.has_issues());
        let second = &input[services[1].span.start as usize..services[1].span.end as usize];
        assert!(second.starts_with("service {"), "got: {second:?}");
        assert!(second.contains("b = 2"), "got: {second:?}");
    }

    #[test]
    fn struct_list_field_accepts_array_of_objects() {
        let input = "services = [\n  { a = 1 },\n  { b = 2 },\n]\n";
        let (_, id, body) = parse_fields(input);
        let fields = fields_of(&body, id);
        let mut report = Report::new();
        let mut services: Vec<Located<Probe>> = Vec::new();
        parse_struct_list_field(&mut services, fields.get("services").unwrap(), &mut report);
        assert_eq!(services.len(), 2);
        assert!(!report.has_issues());
        let first = &input[services[0].span.start as usize..services[0].span.end as usize];
        assert_eq!(first, "{ a = 1 }");
    }

    #[test]
    fn struct_list_field_reports_each_bad_array_element() {
        let input = "services = [{ a = 1 }, 42, true]\n";
        let (_, id, body) = parse_fields(input);
        let fields = fields_of(&body, id);
        let mut report = Report::new();
        let mut services: Vec<Located<Probe>> = Vec::new();
        parse_struct_list_field(&mut services, fields.get("services").unwrap(), &mut report);
        assert_eq!(services.len(), 1, "valid elements still parse");
        assert_eq!(report.issues().len(), 2);
        assert_eq!(report.issues()[0].message, "expected object, found number");
        assert_eq!(report.issues()[1].message, "expected object, found bool");
    }

    #[test]
    fn struct_list_field_rejects_scalar_values() {
        let (_, id, body) = parse_fields("services = 1\n");
        let fields = fields_of(&body, id);
        let mut report = Report::new();
        let mut services: Vec<Located<Probe>> = Vec::new();
        parse_struct_list_field(&mut services, fields.get("services").unwrap(), &mut report);
        assert!(services.is_empty());
        assert_eq!(
            report.issues()[0].message,
            "expected block or array of objects, found number"
        );
    }

    #[test]
    fn syntax_error_is_reported_with_location() {
        let mut sources = SourceMap::new();
        let id = sources.add("broken.hcl", "server {\n  port =\n");
        let mut report = Report::new();
        let parsed: Option<Probe> = parse_hcl(&sources, id, &mut report);
        assert!(parsed.is_none());
        assert!(report.has_errors());
        assert!(report.issues()[0].message.starts_with("syntax error:"));
        assert!(report.issues()[0].span.is_some());
    }

    #[test]
    fn unknown_field_reported_at_name_span() {
        let (_, id, body) = parse_fields("hostnme = \"typo\"\n");
        let fields = fields_of(&body, id);
        let mut report = Report::new();
        report_unknown_field(fields.get("hostnme").unwrap(), &mut report);
        assert_eq!(report.issues()[0].message, "unknown field: hostnme");
        assert_eq!(report.issues()[0].span, Some(Span::new(id, 0, 7)));
    }

    #[test]
    fn unknown_block_reported_with_block_label() {
        let (_, id, body) = parse_fields("tsl {\n}\n");
        let fields = fields_of(&body, id);
        let mut report = Report::new();
        report_unknown_field(fields.get("tsl").unwrap(), &mut report);
        assert_eq!(report.issues()[0].message, "unknown block: tsl");
        assert_eq!(report.issues()[0].span, Some(Span::new(id, 0, 3)));
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
