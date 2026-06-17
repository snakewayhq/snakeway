//! Derive macros for confval.
//!
//! Two derives, one per generated trait impl:
//!
//! - [`Spec`] generates `impl confval::format::FromFields` (structural parsing).
//! - [`Config`] generates `impl confval::pipeline::Lower<Spec>` (lowering).
//!
//! Each derive lives in its own module ([`spec`], [`config`]); [`common`] holds
//! the `syn` type-path helpers both share.

mod common;
mod config;
mod spec;

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

/// Generates an `impl confval::format::FromFields` for a spec struct. Parsing
/// is purely structural; the macro never embeds semantic rules. See [`spec`].
#[proc_macro_derive(Spec, attributes(confval))]
pub fn derive_spec(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match spec::expand(&input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// Generates an `impl confval::pipeline::Lower<Spec>` that converts a validated
/// spec into a runtime config. Same-named fields auto-map through `LowerAuto`,
/// `#[confval(nested)]` fields lower through the inner type's own `Lower` impl,
/// and everything else takes an explicit `#[confval(lower(from = ..., with =
/// ...))]`. The generated impl destructures the Spec exhaustively, so a Spec
/// field consumed by nothing, or a `from` target that does not exist, is a
/// compile error. See [`config`].
#[proc_macro_derive(Config, attributes(confval))]
pub fn derive_config(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match config::expand(&input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}
