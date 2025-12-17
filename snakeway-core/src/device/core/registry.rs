use crate::config::{BuiltinDeviceKind, DeviceKind, SnakewayConfig};
use crate::device::builtin::structured_logging::StructuredLoggingDevice;
use crate::device::core::Device;
use crate::device::wasm::wasm_device::WasmDevice;
use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::sync::Arc;

type BuiltinBuilder = fn(&toml::Value) -> Result<Arc<dyn Device>>;

fn build_structured_logging(cfg: &toml::Value) -> anyhow::Result<Arc<dyn Device>> {
    Ok(Arc::new(StructuredLoggingDevice::from_config(cfg)?))
}

fn builtin_builders() -> HashMap<BuiltinDeviceKind, BuiltinBuilder> {
    let mut map = HashMap::new();

    map.insert(
        BuiltinDeviceKind::StructuredLogging,
        build_structured_logging as BuiltinBuilder,
    );

    map
}

pub struct DeviceRegistry {
    devices: Vec<Arc<dyn Device>>,
}

impl DeviceRegistry {
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
        }
    }

    pub fn load_from_config(&mut self, config: &SnakewayConfig) -> Result<()> {
        let builtin_builders = builtin_builders();

        for cfg in &config.devices {
            if !cfg.enabled {
                continue;
            }

            match cfg.kind {
                DeviceKind::Wasm => {
                    let path = cfg
                        .path
                        .as_ref()
                        .ok_or_else(|| anyhow!("WASM device '{}' missing path", cfg.name))?;

                    let device = WasmDevice::load(path)
                        .with_context(|| format!("failed to load WASM device '{}'", cfg.name))?;

                    self.devices.push(Arc::new(device));
                }

                DeviceKind::Builtin => {
                    let kind = cfg
                        .builtin
                        .as_ref()
                        .ok_or_else(|| anyhow!("builtin device '{}' missing type", cfg.name))?;

                    let builder = builtin_builders
                        .get(kind)
                        .ok_or_else(|| anyhow!("unknown builtin device '{}'", cfg.name))?;

                    let device = builder(&cfg.options).with_context(|| {
                        format!("failed to build builtin device '{}'", cfg.name)
                    })?;

                    self.devices.push(device);
                }
            }
        }

        Ok(())
    }

    pub fn all(&self) -> &[Arc<dyn Device>] {
        &self.devices
    }
}
