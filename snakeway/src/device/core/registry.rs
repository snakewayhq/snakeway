use super::Device;
use crate::config::{BuiltinDeviceKind, DeviceKind, SnakewayConfig};
use crate::device::wasm::wasm_device::WasmDevice;
// use crate::device::builtin::structured_logging::StructuredLoggingDevice;

use crate::device::builtin::structured_logging::StructuredLoggingDevice;
use std::sync::Arc;

pub struct DeviceRegistry {
    devices: Vec<Arc<dyn Device>>,
}

impl DeviceRegistry {
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
        }
    }

    pub fn register(&mut self, device: Arc<dyn Device>) {
        self.devices.push(device);
    }

    pub fn load_from_config(&mut self, config: &SnakewayConfig) {
        for cfg in &config.devices {
            if !cfg.enabled {
                log::debug!("Device '{}' is disabled; skipping", cfg.name);
                continue;
            }

            match cfg.kind {
                DeviceKind::Wasm => {
                    let Some(path) = &cfg.path else {
                        log::error!(
                            "Device '{}' is kind=wasm but no path was provided",
                            cfg.name
                        );
                        continue;
                    };

                    match WasmDevice::load(path) {
                        Ok(device) => {
                            log::info!("Loaded WASM device '{}'", cfg.name);
                            self.register(Arc::new(device));
                        }
                        Err(e) => {
                            log::error!("Failed to load WASM device '{}': {e}", cfg.name);
                        }
                    }
                }

                DeviceKind::Builtin => {
                    let Some(kind) = &cfg.builtin else {
                        log::error!(
                            "Device '{}' is kind=builtin but no builtin type was specified",
                            cfg.name
                        );
                        continue;
                    };

                    match kind {
                        BuiltinDeviceKind::StructuredLogging => {
                            log::info!("Loaded builtin device '{}'", cfg.name);
                            self.register(Arc::new(StructuredLoggingDevice::new()));
                        }
                    }
                }
            }
        }
    }

    pub fn all(&self) -> &[Arc<dyn Device>] {
        &self.devices
    }
}
