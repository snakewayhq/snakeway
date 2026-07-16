//! `#[derive(Spec)]`: generates an `impl confval::format::FromFields` that
//! performs structural parsing only.
//!
//! It walks the `Fields` view, matches fields by name, reports unknown and
//! missing fields, and builds the struct. It performs no validation; entity
//! validation lives in named functions that operate on the parsed `Located`
//! values.

mod options;
mod shape;

use options::{FieldOptions, parse_options};
use shape::{FieldShape, Leaf, classify};

use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Expr, Field, Fields, spanned::Spanned};

/// Builds the `FromFields` parser for one `#[derive(Spec)]` struct.
///
/// The strategy is to walk the struct's fields once and, for each field, decide
/// how it should be parsed and emit the matching code fragments. Those fragments
/// collect into four buckets that are stitched together at the end into a single
/// generated `from_fields` function:
///
/// - `slot_decls`: a local variable per field that holds the value once parsed.
/// - `match_arms`: one arm per field name; when that name is seen in the source,
///   the field is parsed into its slot.
/// - `missing_checks`: run after the walk to report any required field that
///   never appeared.
/// - `constructors`: build the final struct from the filled-in slots.
///
/// At the caller's runtime the generated `from_fields` then iterates the fields
/// actually present in the config source, routes each by name into its match
/// arm (reporting any unrecognized name), runs the missing-field checks, and
/// constructs `Self`. It checks only shape and presence, never values; semantic
/// validation happens later, elsewhere.
pub(crate) fn expand(input: &DeriveInput) -> syn::Result<TokenStream2> {
    // The derive only handles structs with named fields. Enums and tuple
    // structs are rejected with a message pointing at the type.
    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new(
            input.ident.span(),
            "#[derive(Spec)] supports structs with named fields; \
             write FromFields by hand for enums",
        ));
    };
    let Fields::Named(fields) = &data.fields else {
        return Err(syn::Error::new(
            input.ident.span(),
            "#[derive(Spec)] requires named fields",
        ));
    };

    let name = &input.ident;
    // The four buckets of generated code fragments, filled in below and spliced
    // into the final `impl` at the end.
    let mut slot_decls = Vec::new();
    let mut match_arms = Vec::new();
    let mut missing_checks = Vec::new();
    let mut constructors = Vec::new();

    for field in &fields.named {
        let ident = field.ident.as_ref().ok_or_else(|| {
            syn::Error::new_spanned(field, "named field is missing an identifier")
        })?;
        let field_name = ident.to_string();
        // Read the field's attributes, work out its parsing shape, and reject
        // a `default` on a shape that cannot honor it (see below).
        let options = parse_options(field)?;
        let shape = classify(field, options.nested)?;
        reject_unsupported_default(field, &shape, &options)?;
        // `slot` is the generated local variable's name, e.g. `__port`. The
        // leading underscores keep it from clashing with the user's own names.
        let slot = format_ident!("__{}", ident);

        // Emit the parsing fragments for this field, tailored to its shape.
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
                                ::confval::source::Located::detached(#expr),
                            )),
                        });
                    }
                    (Some(default), false) => {
                        match_arms.push(quote! { #field_name => #slot = #parser, });
                        let expr = default_expr(default);
                        constructors.push(quote! {
                            #ident: #slot.unwrap_or_else(
                                || ::confval::source::Located::detached(#expr),
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
                            ::confval::format::parse_string_list_field(__field, report),
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
                            #slot = ::confval::format::parse_string_list_field(__field, report);
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
                        ::confval::format::parse_string_list_field(__field, report),
                });
                constructors.push(quote! { #ident: #slot, });
            }
            FieldShape::Nested { optional } => {
                let seen = format_ident!("__{}_seen", ident);
                slot_decls.push(quote! {
                    let mut #slot = ::core::option::Option::None;
                    let mut #seen: ::core::option::Option<::confval::source::Span> =
                        ::core::option::Option::None;
                });
                match_arms.push(quote! {
                    #field_name => ::confval::format::parse_single_struct(
                        &mut #slot, &mut #seen, #field_name, __field, report,
                    ),
                });
                if optional {
                    constructors.push(quote! { #ident: #slot, });
                } else {
                    // A non-optional nested field is a `Located<S>`. With
                    // `#[confval(default)]` an absent block is filled with
                    // `S::default()` and is not reported as missing; without it,
                    // absence is a missing-field error. Either way a
                    // present-but-failed child is replaced with a detached
                    // default so the parent and its siblings still validate: the
                    // child's structural error is already in the report, so the
                    // lowering gate blocks before the placeholder reaches runtime.
                    if options.default.is_none() {
                        missing_checks.push(quote! {
                            if #seen.is_none() {
                                ::confval::format::report_missing_field(
                                    #field_name, fields.enclosing(), report,
                                );
                            }
                        });
                    }
                    constructors.push(quote! {
                        #ident: #slot.unwrap_or_default(),
                    });
                }
            }
            FieldShape::NestedList => {
                slot_decls.push(quote! { let mut #slot = ::std::vec::Vec::new(); });
                match_arms.push(quote! {
                    #field_name => ::confval::format::parse_struct_list_field(
                        &mut #slot, __field, report,
                    ),
                });
                constructors.push(quote! { #ident: #slot, });
            }
        }
    }

    // Splice the four buckets into the generated parser. This is the code that
    // runs at the caller's runtime, once per parsed struct.
    Ok(quote! {
        impl ::confval::format::FromFields for #name {
            fn from_fields(
                fields: &::confval::format::Fields,
                report: &mut ::confval::diagnostic::Report,
            ) -> ::core::option::Option<Self> {
                #(#slot_decls)*

                for __field in fields.iter() {
                    match __field.name.as_str() {
                        #(#match_arms)*
                        _ => ::confval::format::report_unknown_field(__field, report),
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

/// Picks the confval parse function for a leaf type.
///
/// Returns a generated expression that parses the current field into an
/// `Option<Located<T>>`. `PathBuf` has no parser of its own: it is read as a
/// string and converted, so its arm wraps the string parser with a `map`.
fn leaf_parser(leaf: &Leaf) -> TokenStream2 {
    match leaf {
        Leaf::String => quote! { ::confval::format::parse_string_field(__field, report) },
        Leaf::Int => quote! { ::confval::format::parse_int_field(__field, report) },
        Leaf::Float => quote! { ::confval::format::parse_float_field(__field, report) },
        Leaf::Bool => quote! { ::confval::format::parse_bool_field(__field, report) },
        Leaf::PathBuf => quote! {
            ::confval::format::parse_string_field(__field, report)
                .map(|__value| __value.map(::std::path::PathBuf::from))
        },
    }
}

/// The generated expression for a leaf field's default value, used when the
/// field is absent from the source. `#[confval(default = expr)]` uses `expr`;
/// a bare `#[confval(default)]` falls back to the type's `Default`.
fn default_expr(default: &Option<Expr>) -> TokenStream2 {
    match default {
        Some(expr) => quote! { #expr },
        None => quote! { ::core::default::Default::default() },
    }
}

/// Rejects `#[confval(default ...)]` on field shapes that would silently ignore
/// it. Only leaf fields honor a default value; a string list accepts a bare
/// `#[confval(default)]` (meaning "empty") but not `default = <expr>`. Every
/// other shape would drop the default on the floor, so flag it at compile time
/// rather than surprise the author at runtime.
fn reject_unsupported_default(
    field: &Field,
    shape: &FieldShape,
    options: &FieldOptions,
) -> syn::Result<()> {
    let Some(default) = &options.default else {
        return Ok(());
    };
    let supported = match shape {
        FieldShape::Leaf { .. } => true,
        // A string list's only meaningful default is the empty list, written as
        // a bare `#[confval(default)]`. An explicit value cannot be honored.
        FieldShape::BareStringList => default.is_none(),
        // A non-optional nested block may default to its type's `Default` via a
        // bare `#[confval(default)]`; there is no sensible `default = expr` for a
        // whole sub-struct. An optional nested field is already "absent = None",
        // so a default would be meaningless there.
        FieldShape::Nested { optional: false } => default.is_none(),
        FieldShape::Nested { optional: true }
        | FieldShape::NestedList
        | FieldShape::OptionalWrappedStringList => false,
    };
    if supported {
        return Ok(());
    }
    let span = default
        .as_ref()
        .map(Spanned::span)
        .unwrap_or_else(|| field.span());
    Err(syn::Error::new(
        span,
        "#[confval(default)] is not supported here; a leaf field takes \
         #[confval(default)] or #[confval(default = expr)], while a string list \
         or a non-optional nested block accepts only a bare #[confval(default)]",
    ))
}

/// The generated after-the-walk check that reports a required field as missing.
///
/// `seen` is the boolean local the match arm flips to `true` when it parses the
/// field. If the walk finished without ever setting it, the field was absent
/// and an error is reported against the enclosing block.
fn seen_missing_check(field_name: &str, seen: &proc_macro2::Ident) -> TokenStream2 {
    quote! {
        if !#seen {
            ::confval::format::report_missing_field(#field_name, fields.enclosing(), report);
        }
    }
}
