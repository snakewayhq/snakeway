fmt:
    cargo fmt

clippy:
    cargo clippy --all-targets --all-features -- -D warnings
    # Second pass without benchmarks to catch dead code only kept alive by bench targets.
    cargo clippy --workspace --lib --tests --all-features -- -D warnings

check:
    cargo check

machete:
    cargo machete

# Build API docs for every workspace crate, treating rustdoc warnings as errors.
doc:
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps

lint: fmt clippy
