# Create WIT bindings for example WASM device
generate-wit-bindings:
    @echo "Generate bindings for WASM devices"
    wit-bindgen rust ./crates/snakeway-wit/wit \
      --world snakeway \
      --out-dir ./crates/snakeway-wit/src/
