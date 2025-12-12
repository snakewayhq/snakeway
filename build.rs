use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let wit_path = manifest_dir
        .join("../wasm/abi/snakeway.wit")
        .canonicalize()
        .unwrap();

    println!("cargo:rustc-env=SNAKEWAY_WIT_PATH={}", wit_path.display());
}
