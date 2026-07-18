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

# List lines of code (exluding tests) per file
list-lines:
    #!/usr/bin/env bash
    find crates/ -name "*.rs" -type f | while read -r file; do
        lines=$(sed '/#\[cfg(test)\]/,/^[[:space:]]*}/d' "$file" | sed '/^[[:space:]]*$/d' | wc -l)
        echo "$lines $file"
    done | sort -nr | awk '$1 >= 300'
# Build API docs for every workspace crate, treating rustdoc warnings as errors.
doc:
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps

lint: fmt clippy
