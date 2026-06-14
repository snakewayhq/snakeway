//! Derive macros for confval.
//!
//! `#[derive(Spec)]` generates an `impl confval::hcl::FromHcl` that performs
//! structural parsing only: it walks the `Fields` view, matches fields by
//! name, reports unknown and missing fields, and builds the struct. It
//! performs no validation; entity validation lives in named functions that
//! operate on the parsed `Located` values.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Expr, Field, Fields, Type, parse_macro_input, spanned::Spanned};

#[proc_macro_derive(Spec, attributes(confval))]
pub fn derive_spec(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand(&input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// `#[derive(Config)]` generates an `impl confval::provenance::Lower<Spec>`
/// per spec section 9.2: same-named fields auto-map through `LowerAuto`
/// (unwrapping `Located` layers, never narrowing), `#[confval(nested)]`
/// fields lower through the inner type's own `Lower` impl, and everything
/// else takes an explicit `#[confval(lower(from = ..., with = ...))]`.
/// The generated impl destructures the Spec exhaustively, so a Spec field
/// consumed by nothing, or a `from` target that does not exist, is a
/// compile error.
#[proc_macro_derive(Config, attributes(confval))]
pub fn derive_config(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand_config(&input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// Per-field options from `#[confval(...)]`.
struct FieldOptions {
    nested: bool,
    /// `Some(None)` for `default` (type default), `Some(Some(expr))` for
    /// `default = expr`.
    default: Option<Option<Expr>>,
}

/// What the field's type shape means for parsing.
enum FieldShape {
    /// `Located<L>` / `Option<Located<L>>` for a leaf type `L`.
    Leaf { leaf: Leaf, optional: bool },
    /// `Vec<Located<String>>`: parsed as a string list, unwrapped to the
    /// bare vector.
    BareStringList,
    /// `Option<Located<Vec<Located<String>>>>`: optional string list that
    /// keeps the outer `Located`.
    OptionalWrappedStringList,
    /// `Located<S>` / `Option<Located<S>>` where `S: FromHcl`.
    Nested { optional: bool },
    /// `Vec<Located<S>>` where `S: FromHcl`.
    NestedList,
}

enum Leaf {
    String,
    Int,
    Float,
    Bool,
    PathBuf,
}

fn expand(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new(
            input.ident.span(),
            "#[derive(Spec)] supports structs with named fields; \
             write FromHcl by hand for enums",
        ));
    };
    let Fields::Named(fields) = &data.fields else {
        return Err(syn::Error::new(
            input.ident.span(),
            "#[derive(Spec)] requires named fields",
        ));
    };

    let name = &input.ident;
    let mut slot_decls = Vec::new();
    let mut match_arms = Vec::new();
    let mut missing_checks = Vec::new();
    let mut constructors = Vec::new();

    for field in &fields.named {
        let ident = field.ident.as_ref().expect("named field");
        let field_name = ident.to_string();
        let options = parse_options(field)?;
        let shape = classify(field, &options)?;
        let slot = format_ident!("__{}", ident);

        match shape {
            FieldShape::Leaf { leaf, optional } => {
                let parser = leaf_parser(&leaf);
                slot_decls.push(quote! { let mut #slot = ::core::option::Option::None; });
                match (&options.default, optional) {
                    (Some(default), true) => {
                        match_arms.push(quote! { #field_name => #slot = #parser, });
                        let expr = default_expr(default);
                        constructors.push(quote! {
                            #ident: #slot.or_else(|| ::core::option::Option::Some(
                                ::confval::provenance::Located::detached(#expr),
                            )),
                        });
                    }
                    (Some(default), false) => {
                        match_arms.push(quote! { #field_name => #slot = #parser, });
                        let expr = default_expr(default);
                        constructors.push(quote! {
                            #ident: #slot.unwrap_or_else(
                                || ::confval::provenance::Located::detached(#expr),
                            ),
                        });
                    }
                    (None, true) => {
                        match_arms.push(quote! { #field_name => #slot = #parser, });
                        constructors.push(quote! { #ident: #slot, });
                    }
                    (None, false) => {
                        // Required, no default: track presence in the same
                        // single pass that parses, so a present-but-failed
                        // field is not also reported as missing, without an
                        // O(fields) `Fields::has` rescan.
                        let seen = format_ident!("__{}_seen", ident);
                        slot_decls.push(quote! { let mut #seen = false; });
                        match_arms.push(quote! {
                            #field_name => { #seen = true; #slot = #parser; }
                        });
                        missing_checks.push(seen_missing_check(&field_name, &seen));
                        constructors.push(quote! { #ident: #slot?, });
                    }
                }
            }
            FieldShape::BareStringList => {
                slot_decls.push(quote! { let mut #slot = ::core::option::Option::None; });
                if options.default.is_some() {
                    match_arms.push(quote! {
                        #field_name => #slot =
                            ::confval::hcl::parse_string_list_field(__field, report),
                    });
                    constructors.push(quote! {
                        #ident: #slot.map(|__list| __list.value).unwrap_or_default(),
                    });
                } else {
                    let seen = format_ident!("__{}_seen", ident);
                    slot_decls.push(quote! { let mut #seen = false; });
                    match_arms.push(quote! {
                        #field_name => {
                            #seen = true;
                            #slot = ::confval::hcl::parse_string_list_field(__field, report);
                        }
                    });
                    missing_checks.push(seen_missing_check(&field_name, &seen));
                    constructors.push(quote! { #ident: #slot?.value, });
                }
            }
            FieldShape::OptionalWrappedStringList => {
                slot_decls.push(quote! { let mut #slot = ::core::option::Option::None; });
                match_arms.push(quote! {
                    #field_name => #slot =
                        ::confval::hcl::parse_string_list_field(__field, report),
                });
                constructors.push(quote! { #ident: #slot, });
            }
            FieldShape::Nested { optional } => {
                let seen = format_ident!("__{}_seen", ident);
                slot_decls.push(quote! {
                    let mut #slot = ::core::option::Option::None;
                    let mut #seen: ::core::option::Option<::confval::provenance::Span> =
                        ::core::option::Option::None;
                });
                match_arms.push(quote! {
                    #field_name => ::confval::hcl::parse_single_struct(
                        &mut #slot, &mut #seen, #field_name, __field, report,
                    ),
                });
                if optional {
                    constructors.push(quote! { #ident: #slot, });
                } else {
                    missing_checks.push(quote! {
                        if #seen.is_none() {
                            ::confval::hcl::report_missing_field(
                                #field_name, fields.enclosing(), report,
                            );
                        }
                    });
                    // A failed nested child is non-fatal to the parent: it is
                    // replaced with a detached default so the parent and its
                    // siblings still validate. The child's structural error is
                    // already in the report, so the lowering gate still blocks
                    // and the placeholder never reaches runtime.
                    constructors.push(quote! {
                        #ident: #slot.unwrap_or_default(),
                    });
                }
            }
            FieldShape::NestedList => {
                slot_decls.push(quote! { let mut #slot = ::std::vec::Vec::new(); });
                match_arms.push(quote! {
                    #field_name => ::confval::hcl::parse_struct_list_field(
                        &mut #slot, __field, report,
                    ),
                });
                constructors.push(quote! { #ident: #slot, });
            }
        }
    }

    Ok(quote! {
        impl ::confval::hcl::FromHcl for #name {
            fn from_hcl(
                fields: &::confval::hcl::Fields<'_>,
                report: &mut ::confval::provenance::Report,
            ) -> ::core::option::Option<Self> {
                #(#slot_decls)*

                for __field in fields.iter() {
                    match __field.name {
                        #(#match_arms)*
                        _ => ::confval::hcl::report_unknown_field(__field, report),
                    }
                }

                #(#missing_checks)*

                ::core::option::Option::Some(Self {
                    #(#constructors)*
                })
            }
        }
    })
}

fn parse_options(field: &Field) -> syn::Result<FieldOptions> {
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

fn classify(field: &Field, options: &FieldOptions) -> syn::Result<FieldShape> {
    let ty = &field.ty;
    let (optional, inner) = match unwrap_generic(ty, "Option") {
        Some(inner) => (true, inner),
        None => (false, ty),
    };

    if options.nested {
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

fn leaf_parser(leaf: &Leaf) -> TokenStream2 {
    match leaf {
        Leaf::String => quote! { ::confval::hcl::parse_string_field(__field, report) },
        Leaf::Int => quote! { ::confval::hcl::parse_int_field(__field, report) },
        Leaf::Float => quote! { ::confval::hcl::parse_float_field(__field, report) },
        Leaf::Bool => quote! { ::confval::hcl::parse_bool_field(__field, report) },
        Leaf::PathBuf => quote! {
            ::confval::hcl::parse_string_field(__field, report)
                .map(|__value| __value.map(::std::path::PathBuf::from))
        },
    }
}

fn default_expr(default: &Option<Expr>) -> TokenStream2 {
    match default {
        Some(expr) => quote! { #expr },
        None => quote! { ::core::default::Default::default() },
    }
}

fn seen_missing_check(field_name: &str, seen: &proc_macro2::Ident) -> TokenStream2 {
    quote! {
        if !#seen {
            ::confval::hcl::report_missing_field(#field_name, fields.enclosing(), report);
        }
    }
}

fn unwrap_generic<'a>(ty: &'a Type, name: &str) -> Option<&'a Type> {
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

fn last_segment(ty: &Type) -> Option<String> {
    let Type::Path(path) = ty else {
        return None;
    };
    Some(path.path.segments.last()?.ident.to_string())
}

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

fn expand_config(input: &DeriveInput) -> syn::Result<TokenStream2> {
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
                    #ident: ::confval::provenance::LowerAuto::lower_auto(#ident),
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
                                    ::confval::provenance::Lower::lower(&__value.value, report)?,
                                ),
                            ::core::option::Option::None => ::core::option::Option::None,
                        },
                    });
                } else if unwrap_generic(ty, "Vec").is_some() {
                    constructors.push(quote! {
                        #ident: {
                            let mut __out = ::std::vec::Vec::new();
                            for __value in #ident {
                                __out.push(::confval::provenance::Lower::lower(
                                    &__value.value,
                                    report,
                                )?);
                            }
                            __out
                        },
                    });
                } else {
                    constructors.push(quote! {
                        #ident: ::confval::provenance::Lower::lower(&#ident.value, report)?,
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
        quote! { where #spec_type: ::confval::provenance::Validate }
    } else {
        quote! {}
    };

    Ok(quote! {
        impl ::confval::provenance::Lower<#spec_type> for #name #where_clause {
            fn lower(
                spec: &#spec_type,
                report: &mut ::confval::provenance::Report,
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
