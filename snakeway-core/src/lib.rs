pub mod config;
pub mod ctx;
pub mod device;
pub mod http_event;
pub mod logging;
mod proxy;
pub mod server;

// #[allow(unused_imports)]
// pub mod prelude {
//     use super::config::SnakewayConfig;
//     use super::logging::{LogMode, default_log_mode, init_logging};
//     use super::device::core::Device;
//     use super::device::wasm::wasm_device::WasmDevice;
//     use super::ctx::RequestCtx;
// }


//     use snakeway_core::config::SnakewayConfig;
//     use snakeway_core::logging::{LogMode, default_log_mode, init_logging};
//     use snakeway_core::device::core::Device;
//     use snakeway_core::device::wasm::wasm_device::WasmDevice;
//     use snakeway_core::ctx::RequestCtx;