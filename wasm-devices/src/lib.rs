use serde::Deserialize;

// Host ABI (your step 5 JSON ABI)
unsafe extern "C" {
    fn host_log(ptr: *const u8, len: i32);
}

#[derive(Deserialize)]
struct RequestCtxDto {
    path: String,
}

/// SAFETY WARNING:
/// This is pre-release code test code. No memory freeing. No bounds checks. Use of unsafe. Etc.
/// Very simple: always return `1` meaning “short-circuit”
#[unsafe(no_mangle)]
pub extern "C" fn on_request(ptr: i32, len: i32) -> i32 {
    // SAFETY: host guarantees (ptr, len) is a valid UTF-8 JSON blob
    let slice = unsafe { core::slice::from_raw_parts(ptr as *const u8, len as usize) };

    let json = match core::str::from_utf8(slice) {
        Ok(s) => s,
        Err(_) => return 0, // fail open
    };

    let dto: RequestCtxDto = match serde_json::from_str(json) {
        Ok(d) => d,
        Err(_) => return 0, // fail open
    };

    if dto.path == "/__metrics" {
        // 1 = short-circuit
        1
    } else {
        // 0 = continue
        0
    }
}
// Needed but unused
#[unsafe(no_mangle)]
pub extern "C" fn before_proxy(_: i32, _: i32) -> i32 {
    0
}
#[unsafe(no_mangle)]
pub extern "C" fn after_proxy(_: i32, _: i32) -> i32 {
    0
}
#[unsafe(no_mangle)]
pub extern "C" fn on_response(_: i32, _: i32) -> i32 {
    0
}
