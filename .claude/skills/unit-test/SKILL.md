# Skill: unit-test — Writing Unit Tests

This skill documents the conventions used for unit tests throughout the codebase.
Follow these patterns precisely when adding new tests.

## Where Tests Live

### `snakeway-core`

Each module places its tests in a `tests` submodule, declared at the bottom of `mod.rs`:

```rust
#[cfg(test)]
mod tests;
```

For modules with many test cases the `tests` submodule is a **directory**:

```
src/
  net/
    mod.rs          ← declares `#[cfg(test)] mod tests;`
    tests/
      mod.rs        ← re-exports test sub-modules
      network_connection_filter_tests.rs
```

For simpler modules with few tests a single `tests.rs` file alongside `mod.rs` suffices.

### `snakeway-conf`

Validation tests live **inline** at the bottom of each `*_validation.rs` file,
inside a `#[cfg(test)] mod tests { ... }` block. Each validation file is self-contained
with its own helpers and imports -- do not create shared test utility modules.

```
types/specification/
  server/
    server_validation.rs        ← #[cfg(test)] mod tests { ... } at bottom
  ingress/
    bind/
      bind_validation.rs        ← #[cfg(test)] mod tests { ... } at bottom
      redirect_validation.rs    ← #[cfg(test)] mod tests { ... } at bottom
    service/
      service_validation.rs     ← #[cfg(test)] mod tests { ... } at bottom
```

Cross-ingress and cross-device tests (checks that span multiple specs) live inline
in the centralized validation files:

```
validation/single_file/
  ingress.rs                    ← #[cfg(test)] mod tests { ... } at bottom
  device.rs                     ← #[cfg(test)] mod tests { ... } at bottom
```

Cross-file validation tests (checks that span multiple config files) live inline in:

```
validation/multi_file/
  tls.rs                        ← #[cfg(test)] mod tests { ... } at bottom
```

**Key rule:** tests for a `ValidateSpec` implementation go in the same file as
that implementation, not in a separate test directory. Construct the spec directly
and call `.validate()` -- do not wrap in parent types unnecessarily.

## The AAA Pattern

Every test follows **Arrange / Act / Assert** with explicit comments marking each section:

```rust
#[test]
fn denies_request_when_ip_not_in_allowlist() {
    // Arrange
    let cidr = "10.0.0.0/8".parse().unwrap();
    let device = NetworkPolicyDevice {
        cidr_allow: CidrCollection::new(&[cidr]),
        allow_forwarded: true,
        on_invalid_forwarded: OnInvalidForwarded::Ignore,
    };
    let mut ctx = ctx_with_identity(identity(
        IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
        false,
        true,
    ));

    // Act
    let result = device.on_request(&mut ctx);

    // Assert
    matches!(result, DeviceResult::Respond(_));
}
```

The comments `// Arrange`, `// Act`, and `// Assert` are **mandatory** — they are not optional
style decoration. Each section must be separated by a blank line.

## Async Tests

For async code, use `#[tokio::test]` instead of `#[test]`:

```rust
#[tokio::test]
async fn test_no_peer_addr_allow() {
    // Arrange
    let filter = NetworkConnectionFilter {
        on_no_peer_addr: OnNoPeerAddr::Allow,
        ..Default::default()
    };

    // Act
    let result = filter.should_accept(None).await;

    // Assert
    assert!(result);
}
```

## Test Naming

Use `snake_case` names that read as a plain-English sentence describing the behaviour being verified.
Prefer positive, behaviour-focused names:

```
// Good
fn allows_trusted_forwarded_identity()
fn denies_request_when_ip_not_in_allowlist()
fn from_config_sets_runtime_fields_correctly()

// Avoid
fn test_network_policy()      // too vague
fn test_ip_check_case_3()     // not readable
```

## Grouping with Section Headers

Use `//---` separator comments to group related tests within a single file:

```rust
//-----------------------------------------------------------------------------
// CIDR Allow List Tests
//-----------------------------------------------------------------------------

#[tokio::test]
async fn test_cidr_allow_empty_accepts_all() { ... }

#[tokio::test]
async fn test_cidr_allow_ip_in_list() { ... }

//-----------------------------------------------------------------------------
// CIDR Deny List Tests
//-----------------------------------------------------------------------------

#[tokio::test]
async fn test_cidr_deny_ip_in_list() { ... }
```

## Helper Functions

Extract shared setup into private helper functions within the test module rather than
repeating construction logic inline:

```rust
fn allow_all_device() -> NetworkPolicyDevice {
    NetworkPolicyDevice {
        cidr_allow: CidrCollection::default(),
        allow_forwarded: true,
        on_invalid_forwarded: OnInvalidForwarded::Ignore,
    }
}

fn identity(ip: IpAddr, is_forwarded: bool, is_trusted: bool) -> ClientIdentity {
    ClientIdentity {
        ip,
        proxy_chain: vec![],
        is_forwarded,
        is_trusted,
        geo: None,
        ua: None,
    }
}
```

Helper functions must not contain assertions — they are pure setup utilities.

## Assertions

- Use `assert!`, `assert_eq!`, and `matches!` from the standard library for simple checks.
- Use `pretty_assertions::assert_eq!` (already in `Cargo.toml`) when comparing complex structs
  or strings where a diff view is helpful.
- Prefer `matches!(value, Pattern)` over `assert!(matches!(value, Pattern))` for enum variants
  when no message is needed. When a failure message would be useful, write it out with
  `assert!(matches!(...), "explanation")`.
- Assertions should be high quality, meaning they should actually serve to validate the subject under test meaningfully.

## Running Unit Tests

```bash
# Run all unit tests
just test

# Run snakeway-core tests
cargo nextest run -p snakeway-core --features static_files,wasm

# Run snakeway-conf tests
cargo test -p snakeway-conf

# Run a specific test by name
cargo nextest run -p snakeway-core --features static_files,wasm -E 'test(denies_request_when_ip_not_in_allowlist)'
cargo test -p snakeway-conf -- upstream_validation::tests::weight_greater_than_zero
```
