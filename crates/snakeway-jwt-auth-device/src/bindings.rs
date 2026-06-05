use wit_bindgen::generate;

generate!({
    path: "../snakeway-wit/wit/",
    world: "device",
});

pub(crate) use crate::bindings::snakeway::device::{host, types};
