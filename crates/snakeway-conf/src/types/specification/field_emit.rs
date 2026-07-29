//! Constructors for the detached fields a handwritten `ToFields` impl emits.
//!
//! `#[derive(Spec)]` generates its own emission code. A spec with a
//! handwritten `FromFields` (the tagged enums) writes its `ToFields` by hand
//! and builds each field through these helpers so every impl produces the
//! same detached shape.

use confval::format::{Field, Fields, Scalar, Value, ValueKind};
use confval::prelude::Located;
use std::collections::HashMap;
use std::path::Path;

pub(crate) fn string_field(name: &str, value: &str) -> Field {
    scalar_field(name, Scalar::String(value.to_string()))
}

pub(crate) fn int_field(name: &str, value: i64) -> Field {
    scalar_field(name, Scalar::Int(value))
}

pub(crate) fn bool_field(name: &str, value: bool) -> Field {
    scalar_field(name, Scalar::Bool(value))
}

pub(crate) fn path_field(name: &str, value: &Path) -> Field {
    scalar_field(name, Scalar::String(value.to_string_lossy().into_owned()))
}

pub(crate) fn string_list_field<'a>(
    name: &str,
    values: impl IntoIterator<Item = &'a Located<String>>,
) -> Field {
    let elements = values
        .into_iter()
        .map(|item| Value::detached(ValueKind::Scalar(Scalar::String(item.value.clone()))))
        .collect();
    Field::detached_value(name, Value::detached(ValueKind::Seq(elements)))
}

/// Entries are sorted by key so emission is deterministic.
pub(crate) fn string_map_field(name: &str, entries: &HashMap<String, String>) -> Field {
    let mut sorted: Vec<(&String, &String)> = entries.iter().collect();
    sorted.sort_by_key(|(key, _)| key.as_str());
    let fields = sorted
        .into_iter()
        .map(|(key, value)| string_field(key, value))
        .collect();
    Field::detached_value(
        name,
        Value::detached(ValueKind::Map(Fields::detached(fields))),
    )
}

fn scalar_field(name: &str, scalar: Scalar) -> Field {
    Field::detached_value(name, Value::detached(ValueKind::Scalar(scalar)))
}
