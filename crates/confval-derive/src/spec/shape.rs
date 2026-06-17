//! Field-type classification for `#[derive(Spec)]`: maps a field's Rust type
//! to the parsing strategy the codegen will use.

use crate::common::{last_segment, unwrap_generic};
use syn::{Field, spanned::Spanned};

/// What the field's type shape means for parsing.
pub(crate) enum FieldShape {
    /// `Located<L>` / `Option<Located<L>>` for a leaf type `L`.
    Leaf { leaf: Leaf, optional: bool },
    /// `Vec<Located<String>>`: parsed as a string list, unwrapped to the
    /// bare vector.
    BareStringList,
    /// `Option<Located<Vec<Located<String>>>>`: optional string list that
    /// keeps the outer `Located`.
    OptionalWrappedStringList,
    /// `Located<S>` / `Option<Located<S>>` where `S: FromFields`.
    Nested { optional: bool },
    /// `Vec<Located<S>>` where `S: FromFields`.
    NestedList,
}

pub(crate) enum Leaf {
    String,
    Int,
    Float,
    Bool,
    PathBuf,
}

/// Classifies a field's type into a [`FieldShape`]. `nested` is the field's
/// `#[confval(nested)]` flag, which selects between leaf and nested handling.
pub(crate) fn classify(field: &Field, nested: bool) -> syn::Result<FieldShape> {
    let ty = &field.ty;
    let (optional, inner) = match unwrap_generic(ty, "Option") {
        Some(inner) => (true, inner),
        None => (false, ty),
    };

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
