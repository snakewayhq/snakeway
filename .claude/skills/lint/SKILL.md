# Skill: lint — Post-Change Code Style Enforcement

Run this skill after making any code changes to ensure the codebase conforms to
Snakeway's code style requirements before committing.

## Steps

### 1. Format with `rustfmt`

```bash
cargo fmt
```

`cargo fmt` reformats every Rust source file in the workspace in-place.
Run it unconditionally — it is idempotent and fast.

### 2. Lint with Clippy

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

Flags explained:
- `--all-targets` — checks library code, binaries, tests, and examples.
- `--all-features` — enables every Cargo feature flag so no code path is skipped.
- `-- -D warnings` — promotes every Clippy warning to a hard error (matches CI behaviour).

Fix every diagnostic Clippy reports before proceeding. Do **not** suppress warnings
with `#[allow(...)]` unless there is a genuine false-positive — and if you do,
add a comment explaining why.

### 3. Shorthand via `just`

Both steps can be run together with the project's `just` recipe:

```bash
just lint
```

This is equivalent to running `cargo fmt` followed by the full `cargo clippy` invocation above.

## Checklist

- [ ] `cargo fmt` applied — no unformatted files remain.
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` exits with code 0.
- [ ] No `#[allow(...)]` annotations added without an explanatory comment.

## Notes

- Run `just fmt` if you only need to reformat without linting.
- Run `just clippy` if you only need to lint without reformatting.
- Run `just test-everything` to also execute the full test suite and integration tests on top of linting.
