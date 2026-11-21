---
title: Code Style
---

Every change must be formatted and lint-clean before it is committed.
CI treats clippy warnings as hard errors, so run the same checks locally to avoid a failed build.

## Format with rustfmt

```bash
cargo fmt
```

`cargo fmt` reformats every Rust source file in the workspace in place.
Run it unconditionally after making changes.
It is idempotent and fast.

## Lint with Clippy

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

The flags matter:

- `--all-targets` checks library code, binaries, tests, and examples.
- `--all-features` enables every Cargo feature flag so no code path is skipped.
- `-- -D warnings` promotes every clippy warning to a hard error, which matches CI behavior.

Fix every diagnostic clippy reports before proceeding.
Do **not** suppress warnings with `#[allow(...)]` unless there is a genuine false positive.
If you do add one, include a comment explaining why.

## Shorthand via just

Both steps can be run together with a single recipe:

```bash
just lint
```

This is equivalent to running `cargo fmt` followed by the full clippy invocation above.

Related recipes:

- `just fmt` reformats without linting.
- `just clippy` lints without reformatting.
- `just test-everything` additionally runs the full unit and integration test suites on top of linting.

## Checklist

Before committing, confirm:

- [ ] `cargo fmt` applied, with no unformatted files remaining.
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` exits with code 0.
- [ ] No `#[allow(...)]` annotations added without an explanatory comment.
