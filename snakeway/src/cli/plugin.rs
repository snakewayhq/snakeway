use anyhow::{anyhow, Result};
use clap::{Args, Subcommand};
use serde::Serialize;
use std::fs;

use wasmtime::{Caller, Engine, Linker, Module, Store};

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
    let wasm_bytes =
        fs::read(&args.file).map_err(|e| anyhow!("failed to read wasm file {}: {e}", args.file))?;

    let engine = Engine::default();
    let mut store = Store::new(&engine, ());

    let module = Module::new(&engine, wasm_bytes)
        .map_err(|e| anyhow!("failed to compile wasm module: {e}"))?;

    let mut linker = Linker::new(&engine);

    // Host import: env::host_log(ptr,len)
    linker.func_wrap(
        "env",
        "host_log",
        |mut caller: Caller<'_, ()>, ptr: i32, len: i32| {
            let memory = caller
                .get_export("memory")
                .and_then(|e| e.into_memory())
                .expect("guest memory not found");

            let data = memory
                .data(&caller)
                .get(ptr as usize..(ptr + len) as usize)
                .expect("pointer out of range");

            let msg = std::str::from_utf8(data).unwrap_or("<non-utf8>");
            println!("[guest] {msg}");
        },
    )?;

    let instance = linker
        .instantiate(&mut store, &module)
        .map_err(|e| anyhow!("failed to instantiate wasm module: {e}"))?;

    let memory = instance
        .get_memory(&mut store, "memory")
        .ok_or_else(|| anyhow!("wasm module does not export a memory named `memory`"))?;

    // Validate and get hook
    // Signature we expect for all hooks for now: (i32,i32) -> i32
    let hook = args.hook.as_str();
    let func = instance
        .get_typed_func::<(i32, i32), i32>(&mut store, hook)
        .map_err(|_| anyhow!("WASM export `{hook}` missing or wrong signature; expected (i32,i32)->i32"))?;

    // Build DTO and serialize
    let dto = RequestCtxDto {
        path: args.path.clone(),
    };
    let json = serde_json::to_vec(&dto).map_err(|e| anyhow!("failed to serialize ctx dto: {e}"))?;

    // Write JSON at offset 0
    let ptr = 0usize;
    let len = json.len();
    let mem_size = memory.data_size(&store);

    if ptr + len > mem_size {
        return Err(anyhow!(
            "not enough guest memory: need {}, have {}",
            ptr + len,
            mem_size
        ));
    }

    // Avoid borrow conflicts by cloning handles
    let mut st = store;

    memory.write(&mut st, ptr, &json)
        .map_err(|e| anyhow!("failed to write ctx into guest memory: {e}"))?;

    if args.verbose {
        println!("file: {}", args.file);
        println!("hook: {}", hook);
        println!("ctx: {}", String::from_utf8_lossy(&json));
        println!("mem_size: {}", mem_size);
        println!("calling {hook}({ptr}, {len}) ...");
    }

    // Call hook
    let code = func
        .call(&mut st, (ptr as i32, len as i32))
        .map_err(|e| anyhow!("WASM call `{hook}` failed: {e}"))?;

    println!("return_code: {code}");
    println!(
        "meaning: {}",
        match code {
            0 => "Continue",
            1 => "ShortCircuit",
            _ => "Unknown",
        }
    );

    Ok(())
}
