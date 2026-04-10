use super::replay_fixture;
use snakeway_tests::constants::HTTP_REPLAY_OK_RESPONSE;

#[test]
fn chrome_navigation_should_proxy() {
    let resp = replay_fixture("browsers/chrome_navigation.http");

    assert!(resp.contains(HTTP_REPLAY_OK_RESPONSE));
}

#[test]
fn chrome_fetch_should_proxy() {
    let resp = replay_fixture("browsers/chrome_fetch.http");
    assert!(resp.contains(HTTP_REPLAY_OK_RESPONSE));
}

#[test]
fn firefox_navigation_should_proxy() {
    let resp = replay_fixture("browsers/firefox_navigation.http");
    assert!(resp.contains(HTTP_REPLAY_OK_RESPONSE));
}
