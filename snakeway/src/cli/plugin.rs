use crate::device::wasm::bindings::{
    exports::snakeway::device::policy::{Decision, Request},
    Snakeway,
};
use crate::device::wasm::wasm_device::HostState;
use anyhow::{anyhow, Result};
use clap::{Args, Subcommand};
use wasmtime::component::ResourceTable;
use wasmtime::{
    component::{Component, Linker}, Engine,
    Store,
};
use wasmtime_wasi::{p2, WasiCtxBuilder};

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

pub fn run(cmd: PluginCmd) -> Result<()> {
    match cmd {
        PluginCmd::Test(args) => run_test(args),
    }
}

fn run_test(args: PluginTestArgs) -> Result<()> {
    let engine = Engine::default();
    let component = Component::from_file(&engine, &args.file)
        .map_err(|e| anyhow!("failed to load component: {e}"))?;

    // Setup linker
    let mut linker = Linker::new(&engine);
    p2::add_to_linker_sync(&mut linker)?;
    let wasi_ctx = WasiCtxBuilder::new().build();
    let mut store = Store::new(
        &engine,
        HostState {
            table: ResourceTable::new(),
            wasi: wasi_ctx,
        },
    );

    // Setup instance
    let instance = Snakeway::instantiate(&mut store, &component, &linker)
        .map_err(|e| anyhow!("failed to instantiate component: {e}"))?;

    let req = Request {
        // path: args.path.clone(),
        // original_path: args.path.original_uri.path().to_string(),
        // route_path: ctx.route_path.clone(),
        route_path: "".to_string(),
        original_path: "".to_string(),
        headers: vec![
            // ("host".into(), "example.com".into()),
        ],
    };

    let policy = instance.snakeway_device_policy();

    let result = match args.hook.as_str() {
        "on_request" => policy.call_on_request(&mut store, &req)?,
        "before_proxy" => policy.call_before_proxy(&mut store, &req)?,
        other => return Err(anyhow!("unknown hook: {other}")),
    };

    println!(
        "decision: {}",
        match result.decision {
            Decision::Continue => "Continue",
            Decision::Block => "Block",
        }
    );

    println!("patch: {}", result.patch.is_some());

    Ok(())
}
