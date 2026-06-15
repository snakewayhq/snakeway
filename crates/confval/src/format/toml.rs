//! TOML frontend: parses TOML text into the format-neutral [`Fields`] tree.
//!
//! Like [`hcl`](crate::format::hcl), this module's only job is the conversion
//! from a concrete syntax tree — here `toml_edit`'s — to the owned model in
//! [`field`](crate::format::field). It parses through
//! [`ImDocument`](toml_edit::ImDocument), the immutable document type that
//! retains source spans, and emits the same neutral `Fields` every other
//! frontend does, so the leaf parsers, the derive-generated walks, and the
//! hand-written [`FromFields`] impls work against it unchanged.
//!
//! TOML's structural shapes map onto the neutral model as follows:
//!
//! - A `[table]` section becomes a [`FieldKind::Block`], mirroring an HCL
//!   block.
//! - An inline table (`x = { ... }`) becomes a [`FieldKind::Value`] holding a
//!   [`ValueKind::Map`], mirroring an HCL object attribute.
//! - An array of tables (`[[x]]`) becomes one field whose value is a
//!   [`ValueKind::Seq`] of maps, so a `Vec<Located<S>>` nested-list field
//!   lowers from it exactly as it would from an HCL array of objects.
//! - A native datetime, which the neutral model has no scalar for, becomes
//!   [`ValueKind::Other`] and surfaces as an ordinary type mismatch.

use crate::diagnostic::Report;
use crate::format::field::{Field, FieldKind, Fields, FromFields, Scalar, Value, ValueKind};
use crate::source::{SourceId, SourceMap, Span};
use std::ops::Range;
use toml_edit::{ImDocument, InlineTable, Item, Table, Value as TomlValue};

/// Parses one registered source into a `T`, pushing syntax errors and
/// structural problems into the report.
pub fn parse_toml<T: FromFields>(
    sources: &SourceMap,
    id: SourceId,
    report: &mut Report,
) -> Option<T> {
    let Some(source) = sources.get(id) else {
        report
            .error("internal error: parse_toml called with an unregistered source id")
            .emit();
        return None;
    };
    match ImDocument::parse(&source.text) {
        Ok(document) => {
            let enclosing = Span::new(id, 0, source.text.len() as u32);
            let fields = fields_of_table(document.as_table(), enclosing, id, report);
            T::from_fields(&fields, report)
        }
        Err(error) => {
            report
                .error(format!("syntax error: {}", error.message()))
                .at(span_of(error.span(), id))
                .emit();
            None
        }
    }
}

fn span_of(range: Option<Range<usize>>, source: SourceId) -> Span {
    match range {
        Some(range) => Span::new(source, range.start as u32, range.end as u32),
        None => Span::detached(),
    }
}

/// The whole-field span, name and value together. Either end may be missing
/// (an absent toml_edit span); the present one then stands alone.
fn entry_span(name_span: Span, value_span: Span) -> Span {
    if name_span.is_detached() {
        value_span
    } else if value_span.is_detached() {
        name_span
    } else {
        Span::merge(name_span, value_span)
    }
}

/// Normalizes a table's entries into neutral fields. Used for the document
/// root, for `[section]` tables, and for each `[[array]]` element.
fn fields_of_table(
    table: &Table,
    enclosing: Span,
    source: SourceId,
    report: &mut Report,
) -> Fields {
    let mut items = Vec::new();
    for (name, item) in table.iter() {
        let name_span = span_of(
            table.get_key_value(name).and_then(|(key, _)| key.span()),
            source,
        );
        items.push(field_of_item(name, name_span, item, source, report));
    }
    Fields::new(source, enclosing, items)
}

/// Builds one field from a table entry, classifying it as a block (section),
/// an array of tables, or an attribute value.
fn field_of_item(
    name: &str,
    name_span: Span,
    item: &Item,
    source: SourceId,
    report: &mut Report,
) -> Field {
    let value_span = span_of(item.span(), source);
    let kind = if let Some(table) = item.as_table() {
        FieldKind::Block(fields_of_table(table, value_span, source, report))
    } else if let Some(array) = item.as_array_of_tables() {
        let elements = array
            .iter()
            .map(|table| {
                let span = span_of(table.span(), source);
                Value {
                    span,
                    kind: ValueKind::Map(fields_of_table(table, span, source, report)),
                }
            })
            .collect();
        FieldKind::Value(Value {
            span: value_span,
            kind: ValueKind::Seq(elements),
        })
    } else if let Some(value) = item.as_value() {
        FieldKind::Value(value_of_value(value, source, report))
    } else {
        FieldKind::Value(Value {
            span: value_span,
            kind: ValueKind::Other("value"),
        })
    };
    Field {
        name: name.to_string(),
        name_span,
        span: entry_span(name_span, value_span),
        source,
        kind,
    }
}

/// Converts one TOML value into a neutral [`Value`], recursing through arrays
/// and inline tables. A datetime has no neutral scalar and becomes
/// [`ValueKind::Other`].
fn value_of_value(value: &TomlValue, source: SourceId, report: &mut Report) -> Value {
    let span = span_of(value.span(), source);
    let kind = if let Some(string) = value.as_str() {
        ValueKind::Scalar(Scalar::String(string.to_string()))
    } else if let Some(boolean) = value.as_bool() {
        ValueKind::Scalar(Scalar::Bool(boolean))
    } else if let Some(int) = value.as_integer() {
        ValueKind::Scalar(Scalar::Int(int))
    } else if let Some(float) = value.as_float() {
        ValueKind::Scalar(Scalar::Float(float))
    } else if let Some(array) = value.as_array() {
        ValueKind::Seq(
            array
                .iter()
                .map(|element| value_of_value(element, source, report))
                .collect(),
        )
    } else if let Some(inline) = value.as_inline_table() {
        ValueKind::Map(fields_of_inline_table(inline, span, source, report))
    } else if value.is_datetime() {
        ValueKind::Other("datetime")
    } else {
        ValueKind::Other("value")
    };
    Value { span, kind }
}

/// Normalizes an inline table's entries into neutral fields. An inline table
/// holds only values, so every entry is a [`FieldKind::Value`].
fn fields_of_inline_table(
    table: &InlineTable,
    enclosing: Span,
    source: SourceId,
    report: &mut Report,
) -> Fields {
    let mut items = Vec::new();
    for (name, value) in table.iter() {
        let name_span = span_of(
            table.get_key_value(name).and_then(|(key, _)| key.span()),
            source,
        );
        let value = value_of_value(value, source, report);
        let span = entry_span(name_span, value.span);
        items.push(Field {
            name: name.to_string(),
            name_span,
            span,
            source,
            kind: FieldKind::Value(value),
        });
    }
    Fields::new(source, enclosing, items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::field::{
        parse_bool_field, parse_float_field, parse_int_field, parse_string_field,
        parse_string_list_field, parse_struct_field, parse_struct_list_field, report_unknown_field,
    };
    use crate::source::Located;

    struct Probe;
    impl FromFields for Probe {
        fn from_fields(_: &Fields, _: &mut Report) -> Option<Self> {
            Some(Probe)
        }
    }

    fn parse(input: &str) -> (SourceMap, SourceId, Fields) {
        let mut sources = SourceMap::new();
        let id = sources.add("test.toml", input);
        let document = ImDocument::parse(sources.get(id).unwrap().text.clone()).unwrap();
        let mut report = Report::new();
        let fields = fields_of_table(document.as_table(), Span::new(id, 0, 0), id, &mut report);
        assert!(
            !report.has_issues(),
            "frontend reported: {:?}",
            report.issues()
        );
        (sources, id, fields)
    }

    #[test]
    fn string_field_parses_with_value_span() {
        let input = "name = \"api\"\n";
        let (_, _, fields) = parse(input);
        let mut report = Report::new();
        let value = parse_string_field(fields.get("name").unwrap(), &mut report).unwrap();
        assert_eq!(value.value, "api");
        assert_eq!(
            &input[value.span.start as usize..value.span.end as usize],
            "\"api\""
        );
        assert!(!report.has_issues());
    }

    #[test]
    fn int_and_float_are_distinguished() {
        let (_, _, fields) = parse("port = 8080\nratio = 1.5\n");
        let mut report = Report::new();
        assert_eq!(
            parse_int_field(fields.get("port").unwrap(), &mut report)
                .unwrap()
                .value,
            8080
        );
        // A TOML float is not an integer; the int parser rejects it.
        assert!(parse_int_field(fields.get("ratio").unwrap(), &mut report).is_none());
        assert_eq!(report.issues()[0].message, "expected integer, found number");
        assert_eq!(
            parse_float_field(fields.get("ratio").unwrap(), &mut report)
                .unwrap()
                .value,
            1.5
        );
    }

    #[test]
    fn bool_field_parses() {
        let (_, _, fields) = parse("daemon = true\n");
        let mut report = Report::new();
        assert!(
            parse_bool_field(fields.get("daemon").unwrap(), &mut report)
                .unwrap()
                .value
        );
    }

    #[test]
    fn string_list_has_per_element_spans() {
        let input = "allow = [\"10.0.0.0/8\", \"192.168.0.0/16\"]\n";
        let (_, _, fields) = parse(input);
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
    fn section_parses_as_block() {
        let input = "[tls]\ncert = \"a.pem\"\n";
        let (_, _, fields) = parse(input);
        let mut report = Report::new();
        let parsed: Option<Located<Probe>> =
            parse_struct_field(fields.get("tls").unwrap(), &mut report);
        assert!(parsed.is_some());
        assert!(!report.has_issues());
    }

    #[test]
    fn inline_table_parses_as_object() {
        let (_, _, fields) = parse("tls = { cert = \"a.pem\" }\n");
        let mut report = Report::new();
        let FieldKind::Value(value) = &fields.get("tls").unwrap().kind else {
            panic!("expected attribute value");
        };
        let ValueKind::Map(inner) = &value.kind else {
            panic!("expected inline table to become a map");
        };
        assert!(inner.get("cert").is_some());
        let parsed: Option<Located<Probe>> =
            parse_struct_field(fields.get("tls").unwrap(), &mut report);
        assert!(parsed.is_some());
    }

    #[test]
    fn array_of_tables_lowers_as_nested_list() {
        let input = "[[upstream]]\nendpoint = \"10.0.0.1:9000\"\n[[upstream]]\nendpoint = \"10.0.0.2:9000\"\n";
        let (_, _, fields) = parse(input);
        let mut report = Report::new();
        let mut upstreams: Vec<Located<Probe>> = Vec::new();
        parse_struct_list_field(&mut upstreams, fields.get("upstream").unwrap(), &mut report);
        assert_eq!(upstreams.len(), 2);
        assert!(!report.has_issues());
    }

    #[test]
    fn datetime_becomes_other_and_mismatches() {
        // A native TOML datetime has no neutral scalar; it must surface as a
        // type mismatch, not silently parse.
        let (_, _, fields) = parse("when = 1979-05-27T07:32:00Z\n");
        let mut report = Report::new();
        assert!(parse_string_field(fields.get("when").unwrap(), &mut report).is_none());
        assert_eq!(
            report.issues()[0].message,
            "expected string, found datetime"
        );
    }

    #[test]
    fn unknown_field_reported_at_name_span() {
        let (_, id, fields) = parse("hostnme = \"typo\"\n");
        let mut report = Report::new();
        report_unknown_field(fields.get("hostnme").unwrap(), &mut report);
        assert_eq!(report.issues()[0].message, "unknown field: hostnme");
        assert_eq!(report.issues()[0].span, Some(Span::new(id, 0, 7)));
    }

    #[test]
    fn syntax_error_is_reported_with_location() {
        let mut sources = SourceMap::new();
        let id = sources.add("broken.toml", "port = \n");
        let mut report = Report::new();
        let parsed: Option<Probe> = parse_toml(&sources, id, &mut report);
        assert!(parsed.is_none());
        assert!(report.has_errors());
        assert!(report.issues()[0].message.starts_with("syntax error:"));
    }
}
