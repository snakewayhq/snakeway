# Check hygiene beyond the lint recipe: duplication, unused dependencies, formatting, and file size
check-code-quality:
    #!/usr/bin/env bash
    set -euo pipefail
    # The duplication thresholds are a ratchet. They sit slightly above the state
    # at adoption and tighten as duplicate groups are removed.
    cargo dupes --exclude tests --exclude benches --exclude examples --exclude-tests check --max-exact 80 --max-near 8 --max-exact-percent 16.0 --max-near-percent 1.2
    cargo machete
    cargo fmt --check
    fail=0
    while IFS= read -r f; do
      line=$(grep -n '#\[cfg(test)\]\|#\[cfg(all(test' "$f" | head -1 | cut -d: -f1 || true)
      if [ -n "$line" ]; then app=$((line - 1)); else app=$(wc -l < "$f"); fi
      if [ "$app" -gt 600 ]; then echo "file over the 600 application-line hard limit: $f ($app lines)" >&2; fail=1; fi
    done < <(find crates -path '*/src/*' -name '*.rs' ! -path 'crates/snakeway-wit/src/*')
    [ "$fail" -eq 0 ] || exit 1
    echo "check-code-quality passed"

fmt:
    cargo fmt

clippy:
    cargo clippy --workspace --all-targets --all-features -- -D warnings
    # Second pass without benchmarks to catch dead code only kept alive by bench targets.
    cargo clippy --workspace --lib --tests --all-features -- -D warnings

check:
    cargo check

machete:
    cargo machete

# List lines of code (excluding tests) per file
list-lines:
    #!/usr/bin/env bash
    find crates/ -name "*.rs" -type f | while read -r file; do
        lines=$(sed '/#\[cfg(test)\]/,$d' "$file" | sed '/^[[:space:]]*$/d' | wc -l)
        echo "$lines $file"
    done | sort -nr | awk '$1 >= 300'

# Build API docs for every workspace crate, treating rustdoc warnings as errors.
doc:
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps

lint: fmt clippy
