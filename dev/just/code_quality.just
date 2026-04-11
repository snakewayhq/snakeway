fmt:
    cargo fmt

clippy:
    cargo clippy --all-targets --all-features -- -D warnings

check:
    cargo check

lint: fmt clippy
