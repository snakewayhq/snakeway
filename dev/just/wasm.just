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

# Build the WASM test device fixture for integration tests
build-wasm-test-device:
    cargo component build --release --target wasm32-unknown-unknown -p snakeway-wasm-test-device
    mkdir -p crates/snakeway-tests/fixtures/wasm
    cp target/wasm32-unknown-unknown/release/snakeway_wasm_test_device.wasm \
       crates/snakeway-tests/fixtures/wasm/test_device.wasm

# Build the JWT auth WASM device fixture for integration tests
build-wasm-jwt-device:
    cargo component build --release --target wasm32-unknown-unknown -p snakeway-jwt-auth-device
    mkdir -p crates/snakeway-tests/fixtures/wasm
    cp target/wasm32-unknown-unknown/release/snakeway_jwt_auth_device.wasm \
       crates/snakeway-tests/fixtures/wasm/jwt_auth_device.wasm

# Build the WASM fault device fixtures (trap, loop, memory growth) for integration tests. One fixture crate yields one component per feature build.
build-wasm-fault-devices:
    mkdir -p crates/snakeway-tests/fixtures/wasm
    cargo component build --release --target wasm32-unknown-unknown -p snakeway-wasm-fault-devices --no-default-features --features trap
    cp target/wasm32-unknown-unknown/release/snakeway_wasm_fault_devices.wasm \
       crates/snakeway-tests/fixtures/wasm/trap_device.wasm
    cargo component build --release --target wasm32-unknown-unknown -p snakeway-wasm-fault-devices --no-default-features --features loop
    cp target/wasm32-unknown-unknown/release/snakeway_wasm_fault_devices.wasm \
       crates/snakeway-tests/fixtures/wasm/loop_device.wasm
    cargo component build --release --target wasm32-unknown-unknown -p snakeway-wasm-fault-devices --no-default-features --features grow
    cp target/wasm32-unknown-unknown/release/snakeway_wasm_fault_devices.wasm \
       crates/snakeway-tests/fixtures/wasm/grow_device.wasm
