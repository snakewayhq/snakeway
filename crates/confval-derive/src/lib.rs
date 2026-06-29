//! Derive macros for confval.
//!
//! This crate provides two `#[derive(...)]` attributes that the rest of the
//! codebase puts on its config structs:
//!
//! - `#[derive(Spec)]` writes the code that *parses* a struct out of a config
//!   file (an `impl confval::format::FromFields`).
//! - `#[derive(Config)]` writes the code that *converts* a parsed spec into the
//!   runtime form the proxy actually uses (an `impl confval::pipeline::Lower`).
//!
//! ## How a derive macro works (the mental model for this whole crate)
//!
//! A derive macro is a compile-time function. The compiler hands it the source
//! tokens of the struct the user annotated, and the macro returns new tokens
//! (here, a trait `impl`) that the compiler then compiles as if the user had
//! written them by hand.
//!
//! So there are two layers to keep straight while reading this code:
//!
//! - The plain Rust here runs *at compile time*, inspecting the struct.
//! - The code inside the `quote! { ... }` blocks is *generated* text; it does
//!   not run now. It runs later, whenever the generated `impl` is actually
//!   called at the program's runtime.
//!
//! Most of the work is therefore "looking at a field and deciding which snippet
//! of code to emit for it."
//!
//! The two derives are independent. Each lives in its own module (`spec`,
//! `config`); `common` holds the small `syn` type-inspection helpers they
//! both use.

mod common;
mod config;
mod spec;

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

/// Entry point for `#[derive(Spec)]`.
///
/// Parses the annotated struct into a syntax tree, hands it to `spec::expand`
/// to build the parsing `impl`, and returns the result. If anything about the
/// struct is unsupported, the error is turned into a normal compile error that
/// points at the offending code.
#[proc_macro_derive(Spec, attributes(confval))]
pub fn derive_spec(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match spec::expand(&input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// Entry point for `#[derive(Config)]`.
///
/// Same shape as [`derive_spec`]: parse the struct, hand it to
/// `config::expand` to build the lowering `impl`, and surface any problem as
/// a compile error. See the `config` module for what the generated lowering does.
#[proc_macro_derive(Config, attributes(confval))]
pub fn derive_config(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match config::expand(&input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}
