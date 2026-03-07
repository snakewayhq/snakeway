use super::replay_fixture;
use integration_tests::constants::HTTP_REPLAY_OK_RESPONSE;

#[test]
fn duplicate_headers_should_proxy() {
    let resp = replay_fixture("headers/duplicate_header.http");
    assert!(resp.contains(HTTP_REPLAY_OK_RESPONSE));
}

#[test]
fn hop_by_hop_headers_should_proxy() {
    let resp = replay_fixture("headers/hop_by_hop.http");
    assert!(resp.contains(HTTP_REPLAY_OK_RESPONSE));
}
