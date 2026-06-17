//! Small helpers for looking at a field's written-out type.
//!
//! Both derives need to reason about types like `Option<Located<String>>` by
//! peeling them one wrapper at a time. These two functions do that peeling.
//! They work purely on the *syntax* of the type as the user wrote it (a `syn`
//! tree), not on any resolved type information, so they match by the name that
//! appears in source.

use syn::Type;

/// Peels one named wrapper off a type.
///
/// If `ty` is written as `Name<Inner>` with exactly one type inside the angle
/// brackets, returns `Inner`; otherwise returns `None`. Calling
/// `unwrap_generic(ty, "Option")` on `Option<Located<String>>` yields
/// `Located<String>`. The derives chain these calls to walk down through the
/// `Option` / `Vec` / `Located` layers a field type is built from.
///
/// Matching is by the last path segment's name, so `Option<T>` and
/// `std::option::Option<T>` both match `"Option"`.
pub(crate) fn unwrap_generic<'a>(ty: &'a Type, name: &str) -> Option<&'a Type> {
    let Type::Path(path) = ty else {
        return None;
    };
    let segment = path.path.segments.last()?;
    if segment.ident != name {
        return None;
    }
    let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };
    if arguments.args.len() != 1 {
        return None;
    }
    match arguments.args.first()? {
        syn::GenericArgument::Type(inner) => Some(inner),
        _ => None,
    }
}

/// The bare name at the end of a type path.
///
/// Returns the last segment's identifier as a string, so both `String` and
/// `std::string::String` come back as `"String"`. The classifier uses this to
/// recognize the handful of leaf types it supports (`String`, `i64`, `bool`,
/// and so on) once the wrappers have been peeled away.
pub(crate) fn last_segment(ty: &Type) -> Option<String> {
    let Type::Path(path) = ty else {
        return None;
    };
    Some(path.path.segments.last()?.ident.to_string())
}
