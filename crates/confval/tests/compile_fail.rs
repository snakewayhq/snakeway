//! UI tests pinning the derive macros' compile-time diagnostics.
//!
//! Each file under `tests/ui/fail/` must fail to compile with exactly the
//! output in its `.stderr` sibling; files under `tests/ui/pass/` must
//! compile. Re-bless expected output with `TRYBUILD=overwrite cargo test`.

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/pass/*.rs");
    t.compile_fail("tests/ui/fail/*.rs");
}
