# Create WIT bindings for example WASM device
generate-wit-bindings:
    @echo "Generate bindings for WASM devices"
    wit-bindgen rust ./crates/snakeway-wit/wit \
      --world device \
      --out-dir ./crates/snakeway-wit/src/

# Build the JWT auth WASM device and copy to plugins/
build-jwt-auth-device:
    cargo component build --release --target wasm32-unknown-unknown -p snakeway-jwt-auth-device
    mkdir -p plugins
    cp target/wasm32-unknown-unknown/release/snakeway_jwt_auth_device.wasm plugins/
