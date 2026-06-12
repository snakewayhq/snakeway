use hcl::format::Formatter;
use hcl::ser::Serializer;
use serde::Serialize;

pub(crate) fn to_hcl_string<T: Serialize>(value: &T) -> Result<String, hcl::Error> {
    let mut buf = Vec::new();

    let formatter = Formatter::builder().prefer_ident_keys(true).build(&mut buf);

    let mut serializer = Serializer::with_formatter(formatter);
    serializer.serialize(value)?;

    Ok(String::from_utf8(buf).expect("formatter emits valid UTF-8"))
}

/// Serializes a value as HCL using block syntax for nested structures
/// (`server { ... }`), the form the span-first config parser requires,
/// rather than the serializer's attribute-object form (`server = { ... }`).
///
/// Object-valued fields become blocks, arrays of objects become repeated
/// blocks, and everything else stays an attribute.
pub(crate) fn to_hcl_block_string<T: Serialize>(value: &T) -> Result<String, hcl::Error> {
    match hcl::to_value(value)? {
        hcl::Value::Object(object) => hcl::format::to_string(&object_to_body(object)),
        other => to_hcl_string(&other),
    }
}

fn object_to_body(object: hcl::Map<String, hcl::Value>) -> hcl::Body {
    let mut builder = hcl::Body::builder();
    for (key, value) in object {
        builder = match value {
            hcl::Value::Object(nested) => builder.add_block(object_to_block(&key, nested)),
            hcl::Value::Array(items)
                if !items.is_empty() && items.iter().all(|item| item.is_object()) =>
            {
                let mut builder = builder;
                for item in items {
                    if let hcl::Value::Object(nested) = item {
                        builder = builder.add_block(object_to_block(&key, nested));
                    }
                }
                builder
            }
            other => builder.add_attribute(hcl::Attribute::new(
                hcl::Identifier::sanitized(&key),
                hcl::Expression::from(other),
            )),
        };
    }
    builder.build()
}

fn object_to_block(ident: &str, object: hcl::Map<String, hcl::Value>) -> hcl::Block {
    hcl::Block {
        identifier: hcl::Identifier::sanitized(ident),
        labels: Vec::new(),
        body: object_to_body(object),
    }
}
