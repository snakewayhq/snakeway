//! `#[derive(Config)]`: generates an `impl confval::pipeline::Lower<Spec>`.
//!
//! Same-named fields auto-map through `LowerAuto` (unwrapping `Located` layers,
//! never narrowing), `#[confval(nested)]` fields lower through the inner type's
//! own `Lower` impl, and everything else takes an explicit
//! `#[confval(lower(from = ..., with = ...))]`. The generated impl destructures
//! the Spec exhaustively, so a Spec field consumed by nothing, or a `from`
//! target that does not exist, is a compile error.

use crate::common::unwrap_generic;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Data, DeriveInput, Expr, Field, Fields, spanned::Spanned};

/// How one Config field obtains its value.
enum ConfigFieldSource {
    /// Same-named Spec field through `LowerAuto`.
    Auto,
    /// Same-named Spec field through the inner type's `Lower` impl.
    /// The occurrence shape is read off the Config field's type.
    Nested,
    /// Explicit lowering function over one or more Spec fields.
    With {
        from: Vec<syn::Ident>,
        with: syn::Path,
    },
}

pub(crate) fn expand(input: &DeriveInput) -> syn::Result<TokenStream2> {
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

    let mut consumed: Vec<syn::Ident> = Vec::new();
    let mut constructors = Vec::new();

    for field in &fields.named {
        let ident = field.ident.as_ref().expect("named field");
        let source = parse_config_field_options(field)?;

        match source {
            ConfigFieldSource::Auto => {
                push_consumed(&mut consumed, ident.clone());
                constructors.push(quote! {
                    #ident: ::confval::pipeline::LowerAuto::lower_auto(#ident),
                });
            }
            ConfigFieldSource::Nested => {
                push_consumed(&mut consumed, ident.clone());
                let ty = &field.ty;
                if unwrap_generic(ty, "Option").is_some() {
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

fn push_consumed(consumed: &mut Vec<syn::Ident>, ident: syn::Ident) {
    if !consumed.contains(&ident) {
        consumed.push(ident);
    }
}

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

fn parse_config_field_options(field: &Field) -> syn::Result<ConfigFieldSource> {
    let mut source = ConfigFieldSource::Auto;
    for attr in &field.attrs {
        if !attr.path().is_ident("confval") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("nested") {
                source = ConfigFieldSource::Nested;
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
                source = ConfigFieldSource::With { from, with };
                Ok(())
            } else {
                Err(meta.error("unknown confval attribute; expected `nested` or `lower(...)`"))
            }
        })?;
    }
    Ok(source)
}

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
