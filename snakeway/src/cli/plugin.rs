use anyhow::{anyhow, Result};
use clap::{Args, Subcommand};
use serde::Serialize;
use wasmtime::{
    Engine,
    component::{Component, Linker},
    Store,
};

use crate::device::wasm::bindings::{
    Snakeway,
    exports::snakeway::device::device::{Decision, Request},
};

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

    /// Print verbose info
    #[arg(long)]
    pub verbose: bool,
}

#[derive(Serialize)]
struct RequestCtxDto {
    path: String,
}

pub fn run(cmd: PluginCmd) -> Result<()> {
    match cmd {
        PluginCmd::Test(args) => run_test(args),
    }
}

fn run_test(args: PluginTestArgs) -> Result<()> {
    let engine = Engine::default();
    let component = Component::from_file(&engine, &args.file)
        .map_err(|e| anyhow!("failed to load component: {e}"))?;

    let linker = Linker::new(&engine);
    let mut store = Store::new(&engine, ());

    let instance = Snakeway::instantiate(&mut store, &component, &linker)
        .map_err(|e| anyhow!("failed to instantiate component: {e}"))?;

    let req = Request {
        path: args.path.clone(),
    };

    let decision = instance
        .snakeway_device_device()
        .call_on_request(&mut store, &req)
        .map_err(|e| anyhow!("on_request failed: {e}"))?;

    println!(
        "decision: {}",
        match decision {
            Decision::Continue => "Continue",
            Decision::Block => "Block",
        }
    );

    Ok(())
}