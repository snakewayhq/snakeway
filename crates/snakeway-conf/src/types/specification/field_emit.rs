//! Constructors for detached field shapes `FieldsBuilder` does not cover.
//!
//! The handwritten `ToFields` impls build their walks with confval's
//! `FieldsBuilder`. The string-keyed map is the one shape the builder leaves
//! to the caller, so its constructor lives here.

use confval::format::{Field, Fields, Scalar, Value, ValueKind};
use std::collections::HashMap;

/// Entries are sorted by key so emission is deterministic.
pub(crate) fn string_map_field(name: &str, entries: &HashMap<String, String>) -> Field {
    let mut sorted: Vec<(&String, &String)> = entries.iter().collect();
    sorted.sort_by_key(|(key, _)| key.as_str());
    let fields = sorted
        .into_iter()
        .map(|(key, value)| {
            Field::detached_value(
                key,
                Value::detached(ValueKind::Scalar(Scalar::String(value.to_string()))),
            )
        })
        .collect();
    Field::detached_value(
        name,
        Value::detached(ValueKind::Map(Fields::detached(fields))),
    )
}
