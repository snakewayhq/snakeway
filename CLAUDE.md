# Snakeway — Claude Code Notes

## Environment constraints

### `protoc` is not installed

The `integration-tests` crate requires the Protocol Buffers compiler (`protoc`) to
compile its gRPC stubs. **`protoc` is not available in this environment and cannot
be installed.** Do not attempt to install it.

Consequence: `cargo check --workspace` and `cargo test --workspace` will always fail
for the `integration-tests` crate with:

```
thread 'main' panicked at integration-tests/build.rs:...:
failed to compile protos: Custom { kind: NotFound, error: "Could not find `protoc`" }
```

**This is a pre-existing environment issue, not caused by code changes.**

To verify that your own changes compile correctly, scope the check to the relevant
packages instead:

```sh
cargo check -p snakeway-core -p snakeway
cargo test  -p snakeway-core -p snakeway
```
