fmt:
    cargo fmt

clippy:
    cargo clippy --all-targets --all-features -- -D warnings
    # Second pass without benchmarks to catch dead code only kept alive by bench targets.
    cargo clippy --workspace --exclude snakeway-tests --lib --tests --all-features -- -D warnings

check:
    cargo check

machete:
    cargo machete

lint: fmt clippy
