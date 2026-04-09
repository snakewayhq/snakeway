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
