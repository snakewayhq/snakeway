---
title: Unit Tests
---

This page documents the conventions for unit tests in the library crates (`snakeway` and `snakeway-conf`).
Follow these patterns precisely when adding new tests.

## Where Tests Live

Unit tests live **inline** at the bottom of the file they test, inside a `#[cfg(test)] mod tests { ... }` block.
This is the idiomatic Rust convention.

```rust
// At the bottom of the file being tested:
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn my_test() { ... }
}
```

Do not create separate `tests.rs` files or `tests/` submodule directories for new tests.
Existing test submodules in the codebase may follow an older pattern.
New tests should always be inline.

Construct types directly and call their methods.
Do not wrap the subject under test in parent types unnecessarily.

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

The comments `// Arrange`, `// Act`, and `// Assert` are **mandatory**.
They are not optional style decoration.
Separate each section with a blank line.

The Act section should contain a single line unless the structure of what is being tested requires more.

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

Use `snake_case` names that read as a plain-English sentence describing the behavior being verified.
Prefer positive, behavior-focused names:

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

Extract shared setup into private helper functions within the test module rather than repeating construction logic
inline:

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

Helper functions must not contain assertions.
They are pure setup utilities.

## Assertions

- Use `assert!`, `assert_eq!`, and `matches!` from the standard library for simple checks.
- Use `pretty_assertions::assert_eq!` (already in `Cargo.toml`) when comparing complex structs or strings where a diff
  view is helpful.
- Prefer `matches!(value, Pattern)` for enum variants when no message is needed.
  When a failure message would be useful, write it out with `assert!(matches!(...), "explanation")`.
- Assertion quality matters.
  A stronger, more complete set of assertions that meaningfully validates the subject under test is better than a
  minimal one.

## Running Unit Tests

```bash
# Run all unit tests
just test

# Run snakeway tests
cargo nextest run -p snakeway --all-features

# Run snakeway-conf tests
cargo test -p snakeway-conf

# Run a specific test by name
cargo nextest run -p snakeway --all-features -E 'test(denies_request_when_ip_not_in_allowlist)'
cargo test -p snakeway-conf -- upstream_validation::tests::weight_greater_than_zero

# Run unit tests with coverage (HTML report, or a terminal summary)
just test-with-coverage
just test-with-coverage-summary
```
