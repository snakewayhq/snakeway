use wasmtime::component::bindgen;

bindgen!({
    path: "../snakeway-wit/wit",
    world: "snakeway",
});
