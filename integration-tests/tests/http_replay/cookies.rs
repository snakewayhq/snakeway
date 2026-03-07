use super::replay_fixture;
use integration_tests::constants::HTTP_REPLAY_OK_RESPONSE;

#[test]
fn large_cookie_should_proxy() {
    let resp = replay_fixture("cookies/large_cookie.http");
    assert!(resp.contains(HTTP_REPLAY_OK_RESPONSE));
}
