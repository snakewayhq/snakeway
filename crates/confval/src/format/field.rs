//! The format-neutral field model.
//!
//! Every concrete format (HCL today, TOML next) parses its own syntax tree and
//! then lowers it into the owned types defined here: a [`Fields`] is one
//! structural level (a body, a table, an inline object), each [`Field`] is one
//! named entry, and a [`Value`] is the data behind it. Once a frontend has
//! produced a `Fields`, nothing downstream — the leaf parsers below, the
//! `#[derive(Spec)]`-generated walks, the hand-written [`FromFields`] impls —
//! knows or cares which format it came from.
//!
//! The model is deliberately owned (no borrow of the format's AST). Config
//! files are small, so the one copy out of the parse tree costs nothing and
//! buys a hard severance from any one format's node types.

use crate::diagnostic::Report;
use crate::source::{Located, SourceId, Span};

/// A scalar leaf: the four value kinds every supported format shares.
///
/// Integers and floats are kept distinct so a format that separates them
/// syntactically (TOML's `1` vs `1.0`) round-trips faithfully; formats with a
/// single number type (HCL) simply classify each literal as one or the other.
#[derive(Debug, Clone, PartialEq)]
pub enum Scalar {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
}

/// The data behind a field, with the span it occupied in source.
#[derive(Debug, Clone)]
pub struct Value {
    pub span: Span,
    pub kind: ValueKind,
}

/// A value is a scalar, a sequence, a nested structure, or something the
/// model cannot represent.
#[derive(Debug, Clone)]
pub enum ValueKind {
    Scalar(Scalar),
    /// An array. Elements keep their own spans so a bad element is reported at
    /// the element, not the whole list.
    Seq(Vec<Value>),
    /// A nested structure spelled inline (an HCL object, a TOML inline table).
    Map(Fields),
    /// Present in source but outside the model: an HCL template or null, a
    /// TOML datetime. The label is the noun diagnostics use ("string
    /// template", "datetime"); no leaf parser matches it, so it always
    /// surfaces as a type mismatch.
    Other(&'static str),
}

/// One named entry at a structural level: an attribute, a block, or a table.
#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    /// Span of the name alone (attribute key, block identifier, table header
    /// key) — where unknown-field errors point.
    pub name_span: Span,
    /// Span of the whole field, name and value together.
    pub span: Span,
    pub source: SourceId,
    pub kind: FieldKind,
}

/// Whether a field was written as an attribute (`name = value`) or as a block
/// (`name { ... }` in HCL, `[name]` / `[[name]]` in TOML).
///
/// Both ultimately carry a nested structure when they name one — a block holds
/// its [`Fields`] directly; an attribute holds a [`Value`] that may be a
/// [`Map`](ValueKind::Map). The distinction is kept only so diagnostics can say
/// "found block" rather than "found object", matching how the operator wrote
/// it.
#[derive(Debug, Clone)]
pub enum FieldKind {
    Value(Value),
    Block(Fields),
}

/// One structural level: the named entries of a body, table, or inline object,
/// plus the span an enclosing-level error (a missing required field) points
/// at.
#[derive(Debug, Clone)]
pub struct Fields {
    source: SourceId,
    enclosing: Span,
    items: Vec<Field>,
}

impl Fields {
    pub fn new(source: SourceId, enclosing: Span, items: Vec<Field>) -> Self {
        Self {
            source,
            enclosing,
            items,
        }
    }

    pub fn source(&self) -> SourceId {
        self.source
    }

    /// The span missing-field errors point at: the surrounding block, or the
    /// whole file at the root.
    pub fn enclosing(&self) -> Span {
        self.enclosing
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Field> {
        self.items.iter()
    }

    pub fn has(&self, name: &str) -> bool {
        self.items.iter().any(|field| field.name == name)
    }

    pub fn get(&self, name: &str) -> Option<&Field> {
        self.items.iter().find(|field| field.name == name)
    }
}

/// Structural construction of `Self` from a neutral field view.
///
/// Implementations walk the fields once, match them by name, and push every
/// problem they find to the report. Returning `None` means at least one error
/// was pushed. This is the trait `#[derive(Spec)]` generates and the one a
/// hand-written spec implements; it names no format.
pub trait FromFields: Sized {
    fn from_fields(fields: &Fields, report: &mut Report) -> Option<Self>;
}

fn describe(value: &Value) -> &'static str {
    match &value.kind {
        ValueKind::Scalar(Scalar::String(_)) => "string",
        ValueKind::Scalar(Scalar::Int(_)) | ValueKind::Scalar(Scalar::Float(_)) => "number",
        ValueKind::Scalar(Scalar::Bool(_)) => "bool",
        ValueKind::Seq(_) => "array",
        ValueKind::Map(_) => "object",
        ValueKind::Other(label) => label,
    }
}

fn report_type_mismatch(value: &Value, expected: &str, report: &mut Report) {
    report
        .error(format!("expected {expected}, found {}", describe(value)))
        .at(value.span)
        .emit();
}

/// Requires the field to be an attribute value, not a block.
fn expect_value<'f>(field: &'f Field, expected: &str, report: &mut Report) -> Option<&'f Value> {
    match &field.kind {
        FieldKind::Value(value) => Some(value),
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
/// anything that is not a string.
pub fn parse_string_field(field: &Field, report: &mut Report) -> Option<Located<String>> {
    let value = expect_value(field, "string", report)?;
    match &value.kind {
        ValueKind::Scalar(Scalar::String(string)) => Some(Located::new(string.clone(), value.span)),
        _ => {
            report_type_mismatch(value, "string", report);
            None
        }
    }
}

/// Parses an integer field. Floats are rejected; narrowing to a smaller width
/// happens at lowering, never here.
pub fn parse_int_field(field: &Field, report: &mut Report) -> Option<Located<i64>> {
    let value = expect_value(field, "integer", report)?;
    match &value.kind {
        ValueKind::Scalar(Scalar::Int(int)) => Some(Located::new(*int, value.span)),
        _ => {
            report_type_mismatch(value, "integer", report);
            None
        }
    }
}

/// Parses a float field. An integer literal widens losslessly, so a whole
/// number is accepted where a float is expected.
pub fn parse_float_field(field: &Field, report: &mut Report) -> Option<Located<f64>> {
    let value = expect_value(field, "number", report)?;
    match &value.kind {
        ValueKind::Scalar(Scalar::Float(float)) => Some(Located::new(*float, value.span)),
        ValueKind::Scalar(Scalar::Int(int)) => Some(Located::new(*int as f64, value.span)),
        _ => {
            report_type_mismatch(value, "number", report);
            None
        }
    }
}

/// Parses a boolean field.
pub fn parse_bool_field(field: &Field, report: &mut Report) -> Option<Located<bool>> {
    let value = expect_value(field, "bool", report)?;
    match &value.kind {
        ValueKind::Scalar(Scalar::Bool(boolean)) => Some(Located::new(*boolean, value.span)),
        _ => {
            report_type_mismatch(value, "bool", report);
            None
        }
    }
}

/// Parses an array-of-strings field with per-element spans, so an invalid
/// element is reported at that element, not at the whole list. Every invalid
/// element is reported; if any element is invalid the field is treated as
/// missing.
pub fn parse_string_list_field(
    field: &Field,
    report: &mut Report,
) -> Option<Located<Vec<Located<String>>>> {
    let value = expect_value(field, "array of strings", report)?;
    let ValueKind::Seq(elements) = &value.kind else {
        report_type_mismatch(value, "array of strings", report);
        return None;
    };
    let mut parsed = Vec::new();
    let mut all_valid = true;
    for element in elements {
        match &element.kind {
            ValueKind::Scalar(Scalar::String(string)) => {
                parsed.push(Located::new(string.clone(), element.span))
            }
            _ => {
                report
                    .error(format!("expected string, found {}", describe(element)))
                    .at(element.span)
                    .emit();
                all_valid = false;
            }
        }
    }
    all_valid.then(|| Located::new(parsed, value.span))
}

/// Parses a nested structure via the inner type's [`FromFields`] impl. Accepts
/// both spellings: a block, or an attribute whose value is a map. The returned
/// `Located` carries the whole structure's span.
pub fn parse_struct_field<S: FromFields>(field: &Field, report: &mut Report) -> Option<Located<S>> {
    match &field.kind {
        FieldKind::Block(fields) => {
            S::from_fields(fields, report).map(|spec| Located::new(spec, field.span))
        }
        FieldKind::Value(value) => match &value.kind {
            ValueKind::Map(fields) => {
                S::from_fields(fields, report).map(|spec| Located::new(spec, value.span))
            }
            _ => {
                report_type_mismatch(value, "block", report);
                None
            }
        },
    }
}

/// Parses a repeated nested structure into `slot`, appending. Accepts both
/// spellings and combinations of them: each repeated block appends one
/// element, and an array-of-maps attribute appends one element per map.
/// Invalid array elements are reported individually and skipped.
pub fn parse_struct_list_field<S: FromFields>(
    slot: &mut Vec<Located<S>>,
    field: &Field,
    report: &mut Report,
) {
    match &field.kind {
        FieldKind::Block(_) => {
            if let Some(parsed) = parse_struct_field(field, report) {
                slot.push(parsed);
            }
        }
        FieldKind::Value(value) => match &value.kind {
            ValueKind::Seq(elements) => {
                for element in elements {
                    match &element.kind {
                        ValueKind::Map(fields) => {
                            if let Some(parsed) = S::from_fields(fields, report) {
                                slot.push(Located::new(parsed, element.span));
                            }
                        }
                        _ => report_type_mismatch(element, "object", report),
                    }
                }
            }
            _ => report_type_mismatch(value, "block or array of objects", report),
        },
    }
}

/// Parses a single-occurrence nested structure into `slot`, tracking the first
/// occurrence in `seen` so a repeated one is reported as a duplicate pointing
/// back at the first. The first occurrence wins.
pub fn parse_single_struct<S: FromFields>(
    slot: &mut Option<Located<S>>,
    seen: &mut Option<Span>,
    name: &str,
    field: &Field,
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
pub fn report_unknown_field(field: &Field, report: &mut Report) {
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

/// Reports a second occurrence of a single-occurrence field, pointing back at
/// the first. The first occurrence wins so parsing can continue.
pub fn report_duplicate_field(name: &str, second: Span, first: Span, report: &mut Report) {
    report
        .error(format!("duplicate field: {name}"))
        .at(second)
        .related(first, "first declared here")
        .emit();
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: SourceId = SourceId(0);

    fn span(start: u32, end: u32) -> Span {
        Span::new(SOURCE, start, end)
    }

    /// An attribute field carrying a scalar value.
    fn scalar_field(name: &str, scalar: Scalar) -> Field {
        Field {
            name: name.to_string(),
            name_span: span(0, name.len() as u32),
            span: span(0, 10),
            source: SOURCE,
            kind: FieldKind::Value(Value {
                span: span(0, 10),
                kind: ValueKind::Scalar(scalar),
            }),
        }
    }

    fn seq_field(name: &str, elements: Vec<ValueKind>) -> Field {
        let values = elements
            .into_iter()
            .map(|kind| Value {
                span: span(0, 1),
                kind,
            })
            .collect();
        Field {
            name: name.to_string(),
            name_span: span(0, name.len() as u32),
            span: span(0, 10),
            source: SOURCE,
            kind: FieldKind::Value(Value {
                span: span(0, 10),
                kind: ValueKind::Seq(values),
            }),
        }
    }

    fn map_value(items: Vec<Field>) -> Value {
        Value {
            span: span(0, 10),
            kind: ValueKind::Map(Fields::new(SOURCE, span(0, 10), items)),
        }
    }

    struct Probe;
    impl FromFields for Probe {
        fn from_fields(_: &Fields, _: &mut Report) -> Option<Self> {
            Some(Probe)
        }
    }

    #[test]
    fn string_field_parses_with_value_span() {
        let field = scalar_field("name", Scalar::String("api".to_string()));
        let mut report = Report::new();
        let value = parse_string_field(&field, &mut report).unwrap();
        assert_eq!(value.value, "api");
        assert_eq!(value.span, span(0, 10));
        assert!(!report.has_issues());
    }

    #[test]
    fn string_field_type_mismatch_reports_and_returns_none() {
        let field = scalar_field("name", Scalar::Int(42));
        let mut report = Report::new();
        assert!(parse_string_field(&field, &mut report).is_none());
        assert_eq!(report.issues()[0].message, "expected string, found number");
    }

    #[test]
    fn int_field_parses_and_rejects_floats() {
        let mut report = Report::new();
        let ok = parse_int_field(&scalar_field("port", Scalar::Int(8080)), &mut report);
        assert_eq!(ok.unwrap().value, 8080);
        assert!(parse_int_field(&scalar_field("port", Scalar::Float(1.5)), &mut report).is_none());
        assert_eq!(report.issues()[0].message, "expected integer, found number");
    }

    #[test]
    fn float_field_parses_floats_and_integers() {
        let mut report = Report::new();
        let ratio = parse_float_field(&scalar_field("ratio", Scalar::Float(0.5)), &mut report);
        let whole = parse_float_field(&scalar_field("whole", Scalar::Int(1)), &mut report);
        assert_eq!(ratio.unwrap().value, 0.5);
        assert_eq!(whole.unwrap().value, 1.0);
        assert!(!report.has_issues());
    }

    #[test]
    fn bool_field_parses() {
        let mut report = Report::new();
        let value = parse_bool_field(&scalar_field("daemon", Scalar::Bool(true)), &mut report);
        assert!(value.unwrap().value);
    }

    #[test]
    fn block_where_scalar_expected_reports_found_block() {
        let field = Field {
            name: "tls".to_string(),
            name_span: span(0, 3),
            span: span(0, 10),
            source: SOURCE,
            kind: FieldKind::Block(Fields::new(SOURCE, span(0, 10), vec![])),
        };
        let mut report = Report::new();
        assert!(parse_string_field(&field, &mut report).is_none());
        assert_eq!(report.issues()[0].message, "expected string, found block");
    }

    #[test]
    fn string_list_field_reports_each_bad_element() {
        let field = seq_field(
            "allow",
            vec![
                ValueKind::Scalar(Scalar::String("ok".to_string())),
                ValueKind::Scalar(Scalar::Int(1)),
                ValueKind::Scalar(Scalar::Bool(true)),
            ],
        );
        let mut report = Report::new();
        assert!(parse_string_list_field(&field, &mut report).is_none());
        assert_eq!(report.issues().len(), 2);
        assert_eq!(report.issues()[0].message, "expected string, found number");
        assert_eq!(report.issues()[1].message, "expected string, found bool");
    }

    #[test]
    fn string_list_field_parses_with_per_element_spans() {
        let field = seq_field(
            "allow",
            vec![
                ValueKind::Scalar(Scalar::String("a".to_string())),
                ValueKind::Scalar(Scalar::String("b".to_string())),
            ],
        );
        let mut report = Report::new();
        let list = parse_string_list_field(&field, &mut report).unwrap();
        assert_eq!(list.value.len(), 2);
        assert_eq!(list.value[0].value, "a");
        assert!(!report.has_issues());
    }

    #[test]
    fn struct_field_accepts_block_and_map_forms() {
        let mut report = Report::new();
        let block = Field {
            name: "tls".to_string(),
            name_span: span(0, 3),
            span: span(0, 10),
            source: SOURCE,
            kind: FieldKind::Block(Fields::new(SOURCE, span(0, 10), vec![])),
        };
        let object = Field {
            name: "tls".to_string(),
            name_span: span(0, 3),
            span: span(0, 10),
            source: SOURCE,
            kind: FieldKind::Value(map_value(vec![])),
        };
        let from_block: Option<Located<Probe>> = parse_struct_field(&block, &mut report);
        let from_object: Option<Located<Probe>> = parse_struct_field(&object, &mut report);
        assert!(from_block.is_some());
        assert!(from_object.is_some());
        assert!(!report.has_issues());
    }

    #[test]
    fn struct_field_rejects_scalars() {
        let field = scalar_field("tls", Scalar::Bool(true));
        let mut report = Report::new();
        let parsed: Option<Located<Probe>> = parse_struct_field(&field, &mut report);
        assert!(parsed.is_none());
        assert_eq!(report.issues()[0].message, "expected block, found bool");
    }

    #[test]
    fn struct_list_field_accepts_array_of_maps() {
        let field = Field {
            name: "services".to_string(),
            name_span: span(0, 8),
            span: span(0, 10),
            source: SOURCE,
            kind: FieldKind::Value(Value {
                span: span(0, 10),
                kind: ValueKind::Seq(vec![map_value(vec![]), map_value(vec![])]),
            }),
        };
        let mut report = Report::new();
        let mut services: Vec<Located<Probe>> = Vec::new();
        parse_struct_list_field(&mut services, &field, &mut report);
        assert_eq!(services.len(), 2);
        assert!(!report.has_issues());
    }

    #[test]
    fn single_struct_reports_repeat_as_duplicate() {
        let mut report = Report::new();
        let mut slot: Option<Located<Probe>> = None;
        let mut seen: Option<Span> = None;

        let first = Field {
            name: "tls".to_string(),
            name_span: span(0, 3),
            span: span(0, 4),
            source: SOURCE,
            kind: FieldKind::Value(map_value(vec![])),
        };
        let second = Field {
            span: span(20, 24),
            ..first.clone()
        };

        parse_single_struct(&mut slot, &mut seen, "tls", &first, &mut report);
        parse_single_struct(&mut slot, &mut seen, "tls", &second, &mut report);

        // First occurrence wins; the repeat is a duplicate pointing back at it.
        assert!(slot.is_some());
        assert_eq!(report.issues().len(), 1);
        assert_eq!(report.issues()[0].message, "duplicate field: tls");
        assert_eq!(report.issues()[0].span, Some(span(20, 24)));
        assert_eq!(
            report.issues()[0].related[0],
            (span(0, 4), "first declared here".to_string())
        );
    }

    #[test]
    fn struct_list_field_reports_each_bad_array_element() {
        let field = seq_field(
            "services",
            vec![
                ValueKind::Scalar(Scalar::Int(42)),
                ValueKind::Scalar(Scalar::Bool(true)),
            ],
        );
        let mut report = Report::new();
        let mut services: Vec<Located<Probe>> = Vec::new();
        parse_struct_list_field(&mut services, &field, &mut report);
        assert!(services.is_empty());
        assert_eq!(report.issues()[0].message, "expected object, found number");
        assert_eq!(report.issues()[1].message, "expected object, found bool");
    }

    #[test]
    fn struct_list_field_rejects_scalar_values() {
        let field = scalar_field("services", Scalar::Int(1));
        let mut report = Report::new();
        let mut services: Vec<Located<Probe>> = Vec::new();
        parse_struct_list_field(&mut services, &field, &mut report);
        assert!(services.is_empty());
        assert_eq!(
            report.issues()[0].message,
            "expected block or array of objects, found number"
        );
    }

    #[test]
    fn unknown_field_and_block_labels() {
        let mut report = Report::new();
        report_unknown_field(
            &scalar_field("hostnme", Scalar::String("typo".to_string())),
            &mut report,
        );
        let block = Field {
            name: "tsl".to_string(),
            name_span: span(0, 3),
            span: span(0, 10),
            source: SOURCE,
            kind: FieldKind::Block(Fields::new(SOURCE, span(0, 10), vec![])),
        };
        report_unknown_field(&block, &mut report);
        assert_eq!(report.issues()[0].message, "unknown field: hostnme");
        assert_eq!(report.issues()[1].message, "unknown block: tsl");
    }

    #[test]
    fn duplicate_field_links_first_occurrence() {
        let mut report = Report::new();
        let first = span(0, 4);
        let second = span(20, 24);
        report_duplicate_field("port", second, first, &mut report);
        let issue = &report.issues()[0];
        assert_eq!(issue.message, "duplicate field: port");
        assert_eq!(issue.span, Some(second));
        assert_eq!(issue.related[0], (first, "first declared here".to_string()));
    }

    #[test]
    fn other_value_describes_with_its_label() {
        let field = Field {
            name: "when".to_string(),
            name_span: span(0, 4),
            span: span(0, 10),
            source: SOURCE,
            kind: FieldKind::Value(Value {
                span: span(0, 10),
                kind: ValueKind::Other("datetime"),
            }),
        };
        let mut report = Report::new();
        assert!(parse_string_field(&field, &mut report).is_none());
        assert_eq!(
            report.issues()[0].message,
            "expected string, found datetime"
        );
    }
}
