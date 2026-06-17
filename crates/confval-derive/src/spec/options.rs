//! Reading the per-field `#[confval(...)]` options for `#[derive(Spec)]`.
//!
//! A spec field can be annotated with `#[confval(nested)]` and/or
//! `#[confval(default)]` / `#[confval(default = expr)]`. This module turns those
//! attributes into a plain [`FieldOptions`] struct the rest of the derive reads.

use syn::{Expr, Field};

/// What a field's `#[confval(...)]` attributes asked for.
pub(crate) struct FieldOptions {
    /// `true` if the field was marked `#[confval(nested)]`, i.e. it is a
    /// sub-struct rather than a scalar.
    pub(crate) nested: bool,
    /// Whether a `default` was requested, and with what value. The two `Option`
    /// layers mean different things:
    ///
    /// - `None`              no `default` attribute at all.
    /// - `Some(None)`        `#[confval(default)]`, use the type's `Default`.
    /// - `Some(Some(expr))`  `#[confval(default = expr)]`, use `expr`.
    pub(crate) default: Option<Option<Expr>>,
}

/// Reads a field's `#[confval(...)]` attributes into [`FieldOptions`].
///
/// Walks every `#[confval(...)]` attribute on the field and records the keys it
/// recognizes. An unrecognized key is a compile error, so a typo like
/// `#[confval(nestd)]` is caught rather than silently ignored.
pub(crate) fn parse_options(field: &Field) -> syn::Result<FieldOptions> {
    let mut options = FieldOptions {
        nested: false,
        default: None,
    };
    for attr in &field.attrs {
        if !attr.path().is_ident("confval") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("nested") {
                options.nested = true;
                Ok(())
            } else if meta.path.is_ident("default") {
                if meta.input.peek(syn::Token![=]) {
                    let expr: Expr = meta.value()?.parse()?;
                    options.default = Some(Some(expr));
                } else {
                    options.default = Some(None);
                }
                Ok(())
            } else {
                Err(meta.error("unknown confval attribute; expected `nested` or `default`"))
            }
        })?;
    }
    Ok(options)
}
