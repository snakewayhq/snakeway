//! WASM fault device fixtures.
//!
//! Each just recipe builds this crate once per feature, so one cdylib yields
//! three component targets: a trapping device, a looping device, and a memory
//! growing device. Tests point a WASM device at the fixture the behavior
//! under test needs.

use wit_bindgen::generate;

generate!({
    path: "../snakeway-wit/wit/",
    world: "device",
});

mod common;

use common::{continue_request, continue_response, passthrough_body};
use exports::snakeway::device::policy::Guest;
use snakeway::device::types::{
    BodyChunk, BodyResult, Request, RequestResult, Response, ResponseResult,
};

/// A device that traps on every request, so tests exercise the WASM failure
/// policies (fail open and fail closed).
#[cfg(feature = "trap")]
#[allow(dead_code)]
mod trap {
    use super::*;

    pub struct Device;

    impl Guest for Device {
        fn on_request(_req: Request) -> RequestResult {
            panic!("trap device: on_request traps");
        }

        fn on_stream_request_body(_req: Request, _chunk: Option<BodyChunk>) -> BodyResult {
            passthrough_body()
        }

        fn before_proxy(_req: Request) -> RequestResult {
            continue_request()
        }

        fn after_proxy(_resp: Response) -> ResponseResult {
            continue_response()
        }

        fn on_stream_response_body(_resp: Response, _chunk: Option<BodyChunk>) -> BodyResult {
            passthrough_body()
        }

        fn on_response(_resp: Response) -> ResponseResult {
            continue_response()
        }
    }
}

/// A device that loops forever on every request, so tests exercise the WASM
/// execution timeout and its failure policy.
#[cfg(feature = "loop")]
#[allow(dead_code)]
mod loop_device {
    use super::*;

    pub struct Device;

    impl Guest for Device {
        fn on_request(_req: Request) -> RequestResult {
            // A loop the optimizer cannot remove: the host's execution
            // timeout is what ends it.
            while !std::hint::black_box(false) {}
            unreachable!("the loop above never ends")
        }

        fn on_stream_request_body(_req: Request, _chunk: Option<BodyChunk>) -> BodyResult {
            passthrough_body()
        }

        fn before_proxy(_req: Request) -> RequestResult {
            continue_request()
        }

        fn after_proxy(_resp: Response) -> ResponseResult {
            continue_response()
        }

        fn on_stream_response_body(_resp: Response, _chunk: Option<BodyChunk>) -> BodyResult {
            passthrough_body()
        }

        fn on_response(_resp: Response) -> ResponseResult {
            continue_response()
        }
    }
}

/// A device whose request handler allocates without bound, so the WASM store
/// memory limit traps it and tests exercise the failure policy.
#[cfg(feature = "grow")]
#[allow(dead_code)]
mod grow {
    use super::*;

    pub struct Device;

    impl Guest for Device {
        fn on_request(_req: Request) -> RequestResult {
            // Allocate one mebibyte at a time until the host's store limit
            // traps the device.
            let mut blocks: Vec<Vec<u8>> = Vec::new();
            loop {
                blocks.push(vec![0u8; 1024 * 1024]);
                std::hint::black_box(&blocks);
            }
        }

        fn on_stream_request_body(_req: Request, _chunk: Option<BodyChunk>) -> BodyResult {
            passthrough_body()
        }

        fn before_proxy(_req: Request) -> RequestResult {
            continue_request()
        }

        fn after_proxy(_resp: Response) -> ResponseResult {
            continue_response()
        }

        fn on_stream_response_body(_resp: Response, _chunk: Option<BodyChunk>) -> BodyResult {
            passthrough_body()
        }

        fn on_response(_resp: Response) -> ResponseResult {
            continue_response()
        }
    }
}

// Feature priority under --all-features: one export wins, and the shipped
// fixtures are always built with --no-default-features, so only one device
// is ever exported in a fixture build.
#[cfg(feature = "grow")]
use grow::Device;
#[cfg(all(feature = "loop", not(feature = "grow")))]
use loop_device::Device;
#[cfg(all(feature = "trap", not(any(feature = "loop", feature = "grow"))))]
use trap::Device;

export!(Device);
