//! `#[derive(Config)]`: generating the step that turns a parsed spec into the
//! runtime form the proxy uses.
//!
//! "Lowering" is that conversion. A spec is the freshly parsed config (every
//! value still wrapped in `Located`, integers still wide, strings still
//! strings); a config is the resolved, typed value the rest of the program
//! runs on. This derive writes an `impl confval::pipeline::Lower<Spec>` that
//! does the conversion field by field:
//!
//! - A field with no attribute auto-maps from the same-named spec field through
//!   `LowerAuto`, which just unwraps the `Located` layers (no narrowing).
//! - A `#[confval(nested)]` field lowers through the inner type's own `Lower`.
//! - Anything else takes an explicit `#[confval(lower(from = ..., with = ...))]`
//!   naming the spec field(s) to read and the function that converts them.
//!
//! The generated code destructures the spec with no `..` rest pattern, which
//! turns two whole classes of mistake into compile errors: a spec field that
//! nothing consumes, and a `from` that names a field which does not exist.

use crate::common::unwrap_generic;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Data, DeriveInput, Expr, Field, Fields, spanned::Spanned};

/// Where one config field gets its value from.
enum ConfigFieldSource {
    /// No attribute: copy from the same-named spec field, unwrapping `Located`.
    Auto,
    /// `#[confval(nested)]`: lower the same-named spec field through its own
    /// `Lower` impl. Whether it is a single value, an `Option`, or a `Vec` is
    /// read off this config field's type.
    ///
    /// `default` is set by `#[confval(nested, default)]` on a non-optional
    /// config field whose spec field is `Option<Located<S>>`: an absent block
    /// lowers `S::default()` instead of producing a missing-field error, so the
    /// runtime field is always populated while the spec stays source-faithful.
    Nested { default: bool },
    /// `#[confval(lower(from = ..., with = ...))]`: call `with` on the named
    /// spec field(s) `from` to produce this field.
    With {
        from: Vec<syn::Ident>,
        with: syn::Path,
    },
}

/// Builds the `Lower` impl for one `#[derive(Config)]` struct.
///
/// Reads the struct-level options (which spec to lower from, and so on), then
/// walks the config fields. For each it works out where the value comes from
/// and emits the line that produces it, recording which spec fields were used.
/// Those lines, plus the list of used spec fields, are assembled into a `lower`
/// function: it destructures the spec by the used field names (the exhaustive
/// destructure that makes an unconsumed field a compile error) and builds the
/// config from the per-field expressions.
pub(crate) fn expand(input: &DeriveInput) -> syn::Result<TokenStream2> {
    // Like the Spec derive, only structs with named fields are supported.
    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new(
            input.ident.span(),
            "#[derive(Config)] supports structs with named fields; \
             write Lower by hand for enums",
        ));
    };
    let Fields::Named(fields) = &data.fields else {
        return Err(syn::Error::new(
            input.ident.span(),
            "#[derive(Config)] requires named fields",
        ));
    };

    let (spec_type, spec_only, validate) = parse_config_struct_options(input)?;
    let name = &input.ident;

    // `consumed` records every spec field a config field reads from, so the
    // generated destructure can name them all. `constructors` is one
    // `field: <expression>` line per config field.
    let mut consumed: Vec<syn::Ident> = Vec::new();
    let mut constructors = Vec::new();

    for field in &fields.named {
        let ident = field.ident.as_ref().ok_or_else(|| {
            syn::Error::new_spanned(field, "named field is missing an identifier")
        })?;
        let source = parse_config_field_options(field)?;

        match source {
            ConfigFieldSource::Auto => {
                push_consumed(&mut consumed, ident.clone());
                constructors.push(quote! {
                    #ident: ::confval::pipeline::LowerAuto::lower_auto(#ident),
                });
            }
            ConfigFieldSource::Nested { default } => {
                push_consumed(&mut consumed, ident.clone());
                let ty = &field.ty;
                if unwrap_generic(ty, "Option").is_some() {
                    if default {
                        return Err(syn::Error::new_spanned(
                            field,
                            "#[confval(nested, default)] is not supported on an optional config \
                             field; an absent block already lowers to `None`",
                        ));
                    }
                    constructors.push(quote! {
                        #ident: match #ident {
                            ::core::option::Option::Some(__value) =>
                                ::core::option::Option::Some(
                                    ::confval::pipeline::Lower::lower(&__value.value, report)?,
                                ),
                            ::core::option::Option::None => ::core::option::Option::None,
                        },
                    });
                } else if unwrap_generic(ty, "Vec").is_some() {
                    if default {
                        return Err(syn::Error::new_spanned(
                            field,
                            "#[confval(nested, default)] is not supported on a list config field",
                        ));
                    }
                    constructors.push(quote! {
                        #ident: {
                            let mut __out = ::std::vec::Vec::new();
                            for __value in #ident {
                                __out.push(::confval::pipeline::Lower::lower(
                                    &__value.value,
                                    report,
                                )?);
                            }
                            __out
                        },
                    });
                } else if default {
                    // The spec field is `Option<Located<S>>`: an absent block
                    // lowers `S::default()` (inferred from the single `Lower`
                    // impl), so a non-optional runtime field is always filled
                    // while the spec keeps recording what the source wrote.
                    constructors.push(quote! {
                        #ident: match #ident {
                            ::core::option::Option::Some(__value) =>
                                ::confval::pipeline::Lower::lower(&__value.value, report)?,
                            ::core::option::Option::None =>
                                ::confval::pipeline::Lower::lower(
                                    &::core::default::Default::default(),
                                    report,
                                )?,
                        },
                    });
                } else {
                    constructors.push(quote! {
                        #ident: ::confval::pipeline::Lower::lower(&#ident.value, report)?,
                    });
                }
            }
            ConfigFieldSource::With { from, with } => {
                for source_field in &from {
                    push_consumed(&mut consumed, source_field.clone());
                }
                constructors.push(quote! {
                    #ident: #with(#(#from,)* report)?,
                });
            }
        }
    }

    let ignored = spec_only.iter().map(|ident| quote! { #ident: _, });

    // The `validate` flag binds the Spec to `Validate`, so a lowerable spec
    // without a validator fails to compile. The bound is opt-in: configs that
    // do not request it lower exactly as before.
    let where_clause = if validate {
        quote! { where #spec_type: ::confval::pipeline::Validate }
    } else {
        quote! {}
    };

    Ok(quote! {
        impl ::confval::pipeline::Lower<#spec_type> for #name #where_clause {
            fn lower(
                spec: &#spec_type,
                report: &mut ::confval::diagnostic::Report,
            ) -> ::core::option::Option<Self> {
                // Exhaustive destructure, no rest pattern: a Spec field
                // consumed by nothing fails compilation here.
                let #spec_type { #(#consumed,)* #(#ignored)* } = spec;
                ::core::option::Option::Some(Self {
                    #(#constructors)*
                })
            }
        }
    })
}

/// Records a spec field as consumed, skipping duplicates. A single spec field
/// can feed several config fields, but the generated destructure must name it
/// only once.
fn push_consumed(consumed: &mut Vec<syn::Ident>, ident: syn::Ident) {
    if !consumed.contains(&ident) {
        consumed.push(ident);
    }
}

/// Reads the struct-level `#[confval(...)]` options.
///
/// Returns the spec type to lower from (`lower_from`, required), the spec-only
/// fields that have no config counterpart (`spec_only`, deliberately ignored
/// during the destructure), and whether the `validate` bound was requested.
/// A missing `lower_from` or an unknown key is a compile error.
fn parse_config_struct_options(
    input: &DeriveInput,
) -> syn::Result<(syn::Path, Vec<syn::Ident>, bool)> {
    let mut spec_type: Option<syn::Path> = None;
    let mut spec_only = Vec::new();
    let mut validate = false;
    for attr in &input.attrs {
        if !attr.path().is_ident("confval") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("lower_from") {
                spec_type = Some(meta.value()?.parse()?);
                Ok(())
            } else if meta.path.is_ident("spec_only") {
                meta.parse_nested_meta(|inner| {
                    let ident = inner
                        .path
                        .get_ident()
                        .ok_or_else(|| inner.error("expected a field name"))?;
                    spec_only.push(ident.clone());
                    Ok(())
                })
            } else if meta.path.is_ident("validate") {
                validate = true;
                Ok(())
            } else {
                Err(meta.error(
                    "unknown confval attribute; expected `lower_from = SpecType`, \
                     `spec_only(...)`, or `validate`",
                ))
            }
        })?;
    }
    spec_type
        .map(|spec_type| (spec_type, spec_only, validate))
        .ok_or_else(|| {
            syn::Error::new(
                input.ident.span(),
                "#[derive(Config)] requires #[confval(lower_from = SpecType)]",
            )
        })
}

/// Reads a config field's `#[confval(...)]` attribute into a
/// [`ConfigFieldSource`].
///
/// A field with no attribute defaults to [`ConfigFieldSource::Auto`].
/// `#[confval(nested)]` selects [`Nested`](ConfigFieldSource::Nested), and
/// `#[confval(lower(from = ..., with = ...))]` selects
/// [`With`](ConfigFieldSource::With) after checking both `from` and `with` were
/// given.
fn parse_config_field_options(field: &Field) -> syn::Result<ConfigFieldSource> {
    let mut nested = false;
    let mut nested_default = false;
    let mut lower: Option<ConfigFieldSource> = None;
    for attr in &field.attrs {
        if !attr.path().is_ident("confval") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("nested") {
                nested = true;
                Ok(())
            } else if meta.path.is_ident("default") {
                nested_default = true;
                Ok(())
            } else if meta.path.is_ident("lower") {
                let mut from = Vec::new();
                let mut with: Option<syn::Path> = None;
                meta.parse_nested_meta(|inner| {
                    if inner.path.is_ident("from") {
                        let expr: Expr = inner.value()?.parse()?;
                        from = from_idents(&expr)?;
                        Ok(())
                    } else if inner.path.is_ident("with") {
                        with = Some(inner.value()?.parse()?);
                        Ok(())
                    } else {
                        Err(inner.error("expected `from = ...` and `with = ...`"))
                    }
                })?;
                if from.is_empty() {
                    return Err(meta.error("`lower` requires `from = <field>` or a tuple"));
                }
                let Some(with) = with else {
                    return Err(meta.error("`lower` requires `with = <function>`"));
                };
                lower = Some(ConfigFieldSource::With { from, with });
                Ok(())
            } else {
                Err(meta.error(
                    "unknown confval attribute; expected `nested`, `default`, or `lower(...)`",
                ))
            }
        })?;
    }

    // `lower(...)` is exclusive with `nested`/`default`: a field either lowers
    // through its own `Lower` impl or through an explicit converter, not both.
    if let Some(with) = lower {
        if nested || nested_default {
            return Err(syn::Error::new_spanned(
                field,
                "#[confval(lower(...))] cannot be combined with `nested` or `default`",
            ));
        }
        return Ok(with);
    }
    if nested {
        return Ok(ConfigFieldSource::Nested {
            default: nested_default,
        });
    }
    if nested_default {
        return Err(syn::Error::new_spanned(
            field,
            "#[confval(default)] requires `nested` on a config field",
        ));
    }
    Ok(ConfigFieldSource::Auto)
}

/// Reads the `from = ...` part of `#[confval(lower(...))]` into a list of spec
/// field names.
///
/// Accepts either a single name (`from = port`) or a tuple of names
/// (`from = (host, port)`) for the case where one config field is built from
/// several spec fields. Anything that is not a bare field name is a compile
/// error.
fn from_idents(expr: &Expr) -> syn::Result<Vec<syn::Ident>> {
    fn path_ident(expr: &Expr) -> syn::Result<syn::Ident> {
        if let Expr::Path(path) = expr
            && let Some(ident) = path.path.get_ident()
        {
            return Ok(ident.clone());
        }
        Err(syn::Error::new(
            expr.span(),
            "expected a Spec field name or a tuple of field names",
        ))
    }

    match expr {
        Expr::Tuple(tuple) => tuple.elems.iter().map(path_ident).collect(),
        other => Ok(vec![path_ident(other)?]),
    }
}
