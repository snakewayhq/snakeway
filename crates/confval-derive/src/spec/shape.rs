//! Working out, from a field's written type, how it should be parsed.
//!
//! Before the code generator can emit a parser for a field, it has to know what
//! kind of thing the field is: a single scalar, a list of strings, a nested
//! sub-struct, and so on. [`classify`] answers that by looking at the field's
//! type and returning a [`FieldShape`]. The generator in the parent module then
//! switches on that shape.

use crate::common::{last_segment, unwrap_generic};
use syn::{Field, spanned::Spanned};

/// The kind of a spec field, which decides how it is parsed.
///
/// Each variant lists the Rust types it covers. `Located<T>` is confval's
/// "value plus its location in the source file" wrapper, and `S` is another
/// `#[derive(Spec)]` struct.
pub(crate) enum FieldShape {
    /// A single scalar value, such as `Located<String>` or
    /// `Option<Located<i64>>`. `optional` is true when wrapped in `Option`.
    Leaf { leaf: Leaf, optional: bool },
    /// A required list of strings written as `Vec<Located<String>>`. The outer
    /// list is unwrapped, so the parsed field is a plain `Vec`.
    BareStringList,
    /// An optional list of strings written as
    /// `Option<Located<Vec<Located<String>>>>`. Unlike the bare form, this
    /// keeps the outer `Located` so the whole list still carries a location.
    OptionalWrappedStringList,
    /// A single nested sub-struct, `Located<S>` or `Option<Located<S>>`, parsed
    /// by recursing into `S`'s own generated parser. `optional` is true when
    /// wrapped in `Option`.
    Nested { optional: bool },
    /// A repeated nested sub-struct, `Vec<Located<S>>` (zero or more blocks).
    NestedList,
}

/// The scalar leaf types confval knows how to parse directly.
pub(crate) enum Leaf {
    String,
    Int,
    Float,
    Bool,
    PathBuf,
}

/// Figures out how a field should be parsed from its written type.
///
/// `nested` is the field's `#[confval(nested)]` flag. When set, the field is
/// expected to be a sub-struct (or list of them) and is matched against the
/// nested shapes; when unset, it must be a leaf scalar or a string list. A type
/// that fits none of the supported shapes is a compile error whose message
/// names what was expected, pointing at the field's type.
pub(crate) fn classify(field: &Field, nested: bool) -> syn::Result<FieldShape> {
    let ty = &field.ty;
    // First peel off an outer `Option`, if there is one. That wrapper is what
    // makes a field optional; everything below classifies what is inside it.
    let (optional, inner) = match unwrap_generic(ty, "Option") {
        Some(inner) => (true, inner),
        None => (false, ty),
    };

    // A `#[confval(nested)]` field must be a sub-struct: either a list of them
    // (`Vec<Located<S>>`) or a single one (`Located<S>`).
    if nested {
        if let Some(vec_inner) = unwrap_generic(inner, "Vec") {
            if optional {
                return Err(syn::Error::new(
                    ty.span(),
                    "nested lists are zero-or-more already; drop the Option",
                ));
            }
            if unwrap_generic(vec_inner, "Located").is_some() {
                return Ok(FieldShape::NestedList);
            }
            return Err(syn::Error::new(
                ty.span(),
                "nested list fields must be Vec<Located<S>>",
            ));
        }
        if unwrap_generic(inner, "Located").is_some() {
            return Ok(FieldShape::Nested { optional });
        }
        return Err(syn::Error::new(
            ty.span(),
            "nested fields must be Located<S>, Option<Located<S>>, or Vec<Located<S>>",
        ));
    }

    // A plain (non-nested) field is normally wrapped in `Located`. Inside that
    // wrapper is either a string list or a single leaf scalar.
    if let Some(located_inner) = unwrap_generic(inner, "Located") {
        if let Some(vec_inner) = unwrap_generic(located_inner, "Vec") {
            if unwrap_generic(vec_inner, "Located")
                .is_some_and(|element| last_segment(element).as_deref() == Some("String"))
            {
                if optional {
                    return Ok(FieldShape::OptionalWrappedStringList);
                }
                return Err(syn::Error::new(
                    ty.span(),
                    "use Vec<Located<String>> for a required string list",
                ));
            }
            return Err(syn::Error::new(
                ty.span(),
                "list fields must be Vec<Located<String>>",
            ));
        }
        let leaf = match last_segment(located_inner).as_deref() {
            Some("String") => Leaf::String,
            Some("i64") | Some("HclInt") => Leaf::Int,
            Some("f64") => Leaf::Float,
            Some("bool") => Leaf::Bool,
            Some("PathBuf") => Leaf::PathBuf,
            _ => {
                return Err(syn::Error::new(
                    ty.span(),
                    "unsupported leaf type; expected String, i64, f64, bool, or PathBuf \
                     inside Located, or mark the field #[confval(nested)]",
                ));
            }
        };
        return Ok(FieldShape::Leaf { leaf, optional });
    }

    // The one shape that is not wrapped in `Located`: a required string list is
    // written as a bare `Vec<Located<String>>`.
    if let Some(vec_inner) = unwrap_generic(inner, "Vec")
        && !optional
        && unwrap_generic(vec_inner, "Located")
            .is_some_and(|element| last_segment(element).as_deref() == Some("String"))
    {
        return Ok(FieldShape::BareStringList);
    }

    Err(syn::Error::new(
        ty.span(),
        "unsupported Spec field type; expected Located<T>, Option<Located<T>>, \
         Vec<Located<String>>, or a #[confval(nested)] structure",
    ))
}
