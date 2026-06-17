//! `syn` type-path inspection shared by both derives.

use syn::Type;

/// If `ty` is `Name<Inner>` with a single angle-bracketed type argument,
/// returns `Inner`. Used to peel `Option`, `Vec`, and `Located` layers.
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

/// The final path segment's identifier as a string, e.g. `String` for
/// `std::string::String`.
pub(crate) fn last_segment(ty: &Type) -> Option<String> {
    let Type::Path(path) = ty else {
        return None;
    };
    Some(path.path.segments.last()?.ident.to_string())
}
