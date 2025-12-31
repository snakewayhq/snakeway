use crate::ctx::RequestCtx;
use crate::device::load_wasm_device;
use anyhow::{Result, anyhow};
use clap::{Args, Subcommand};
use std::net::{IpAddr, Ipv4Addr};

#[derive(Subcommand, Debug)]
pub enum PluginCmd {
    /// Test a WASM plugin by invoking its exported hooks with a minimal ctx DTO.
    Test(PluginTestArgs),
}

#[derive(Args, Debug)]
pub struct PluginTestArgs {
    /// Path to the .wasm file
    pub file: String,

    /// Which hook to call (default: on_request)
    #[arg(long, default_value = "on_request")]
    pub hook: String,

    /// Request path to send to the plugin (used by on_request / before_proxy)
    #[arg(long, default_value = "/")]
    pub path: String,
}

pub fn run(cmd: PluginCmd) -> Result<()> {
    match cmd {
        PluginCmd::Test(args) => run_test(args),
    }
}

fn run_test(args: PluginTestArgs) -> Result<()> {
    tracing::info!(
        "Loading WASM device {} with hook {} against path {}",
        args.file,
        args.hook,
        args.path
    );

    let device = load_wasm_device(&args.file)?;

    let ctx = &mut RequestCtx {
        service: Some("some service".to_string()),
        route_path: Some("/some/route".to_string()),
        peer_ip: IpAddr::V4(Ipv4Addr::LOCALHOST),
        ..Default::default()
    };

    tracing::info!("Pre-device Request Context: {:#?}", ctx);
    tracing::info!("Running device hook...");
    let result = match args.hook.as_str() {
        "on_request" => {
            tracing::info!("calling on_request");
            device.on_request(ctx)
        }
        "before_proxy" => {
            tracing::info!("calling before_proxy");
            device.before_proxy(ctx)
        }
        other => {
            tracing::info!("unknown hook: {other}");
            return Err(anyhow!("unknown hook: {other}"));
        }
    };
    tracing::info!("Finished device hook.");
    tracing::info!("Post-device Request Context: {:#?}", ctx);
    tracing::info!("Device Result: {:#?}", result);
    Ok(())
}
