use pretty_assertions::assert_eq;
use reqwest::StatusCode;
use snakeway_core::testing_api::conf::types::OnInvalidForwardedSpec;
use snakeway_tests::conf::ConfigBuilder;
use snakeway_tests::harness::TestServer;

/// The `ForwardingSpec.on_invalid` field controls what happens when a
/// request arrives with an X-Forwarded-For header whose value cannot be
/// parsed as a valid IP address (e.g. `not-a-valid-ip`).
///
/// Two behaviors are possible:
/// - `Deny`   - return 403 Forbidden (the default)
/// - `Ignore` - discard the invalid header and use the real connection IP

//-----------------------------------------------------------------------------
// on_invalid: Deny (default)
//-----------------------------------------------------------------------------

/// When `on_invalid` is set to `Deny` (the default), a request that
/// carries a malformed X-Forwarded-For header must be rejected with 403.
///
/// This is the safe default: when the forwarded IP cannot be validated,
/// the network policy cannot determine whether access should be granted,
/// so it denies the request rather than risking incorrect access control.
#[test]
fn network_policy_denies_invalid_xff_by_default() {
    // Arrange
    let mut np = ConfigBuilder::make_network_policy_device_spec(vec!["0.0.0.0/0"]);
    np.forwarding.allow = true;
    // on_invalid = Deny is the default
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_identity_device(ConfigBuilder::make_identity_device_with_trusted_proxy())
        .with_network_policy(np)
        .build();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Act -- send a syntactically invalid X-Forwarded-For value
    let res = srv
        .get("/api")
        .header("x-forwarded-for", "not-a-valid-ip")
        .send()
        .unwrap();

    // Assert
    assert_eq!(
        res.status(),
        StatusCode::FORBIDDEN,
        "malformed X-Forwarded-For must be denied when on_invalid is Deny"
    );
}

//-----------------------------------------------------------------------------
// on_invalid: Ignore
//-----------------------------------------------------------------------------

/// When `on_invalid` is set to `Ignore`, a request carrying a malformed
/// X-Forwarded-For header must still succeed.
///
/// The invalid header is discarded and the real connection IP (127.0.0.1)
/// is used for policy evaluation instead.  Since the CIDR allow-list is
/// `0.0.0.0/0` the real IP matches and the request is proxied.
///
/// This mode is appropriate when the proxy sits behind infrastructure
/// that may occasionally send incorrect forwarding headers, and dropping
/// those requests would harm availability more than the security risk.
#[test]
fn network_policy_ignores_invalid_xff_when_configured() {
    // Arrange
    let mut np = ConfigBuilder::make_network_policy_device_spec(vec!["0.0.0.0/0"]);
    np.forwarding.allow = true;
    np.forwarding.on_invalid = OnInvalidForwardedSpec::Ignore;
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_identity_device(ConfigBuilder::make_identity_device_with_trusted_proxy())
        .with_network_policy(np)
        .build();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Act -- same malformed header, different policy
    let res = srv
        .get("/api")
        .header("x-forwarded-for", "not-a-valid-ip")
        .send()
        .unwrap();

    // Assert -- invalid XFF ignored; real connection IP (127.0.0.1) used
    assert_eq!(
        res.status(),
        StatusCode::OK,
        "malformed X-Forwarded-For must be ignored and real connection IP used"
    );
}
