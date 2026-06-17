//! Per-field `#[confval(...)]` attribute parsing for `#[derive(Spec)]`.

use syn::{Expr, Field};

/// Per-field options from `#[confval(...)]`.
pub(crate) struct FieldOptions {
    pub(crate) nested: bool,
    /// `Some(None)` for `default` (type default), `Some(Some(expr))` for
    /// `default = expr`.
    pub(crate) default: Option<Option<Expr>>,
}

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
