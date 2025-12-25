use crate::conf::RuntimeConfig;
use crate::conf::types::{BuiltinDeviceKind, DeviceKind};
use crate::device::builtin::identity::IdentityDevice;
use crate::device::builtin::structured_logging::StructuredLoggingDevice;
use crate::device::core::Device;
#[cfg(feature = "wasm")]
use crate::device::wasm::wasm_device::WasmDevice;
use anyhow::{Context, Result, anyhow};
use std::collections::HashMap;
use std::sync::Arc;

type BuiltinBuilder = fn(&toml::Value) -> Result<Arc<dyn Device>>;

fn build_identity(cfg: &toml::Value) -> anyhow::Result<Arc<dyn Device>> {
    Ok(Arc::new(IdentityDevice::from_config(cfg)?))
}

fn build_structured_logging(cfg: &toml::Value) -> anyhow::Result<Arc<dyn Device>> {
    Ok(Arc::new(StructuredLoggingDevice::from_config(cfg)?))
}

fn builtin_builders() -> HashMap<BuiltinDeviceKind, BuiltinBuilder> {
    let mut map = HashMap::new();

    map.insert(
        BuiltinDeviceKind::Identity,
        build_identity as BuiltinBuilder,
    );

    map.insert(
        BuiltinDeviceKind::StructuredLogging,
        build_structured_logging as BuiltinBuilder,
    );

    map
}

pub struct DeviceRegistry {
    devices: Vec<Arc<dyn Device>>,
}

impl Default for DeviceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceRegistry {
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
        }
    }

    pub fn load_from_config(&mut self, cfg: &RuntimeConfig) -> Result<()> {
        let builders = builtin_builders();

        for device_cfg in &cfg.devices {
            if !device_cfg.enabled {
                continue;
            }

            match device_cfg.kind {
                DeviceKind::Wasm => {
                    self.load_wasm_device(device_cfg)?;
                }

                DeviceKind::Builtin => {
                    let kind = device_cfg.builtin.as_ref().ok_or_else(|| {
                        anyhow!("builtin device '{}' missing type", device_cfg.name)
                    })?;

                    let builder = builders
                        .get(kind)
                        .ok_or_else(|| anyhow!("unknown builtin device '{}'", device_cfg.name))?;

                    let device = builder(&device_cfg.config).with_context(|| {
                        format!("failed to build builtin device '{}'", device_cfg.name)
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

#[cfg(feature = "wasm")]
impl DeviceRegistry {
    fn load_wasm_device(&mut self, cfg: &crate::conf::types::DeviceConfig) -> Result<()> {
        let path = cfg
            .path
            .as_ref()
            .ok_or_else(|| anyhow!("WASM device '{}' missing path", cfg.name))?;

        let device = WasmDevice::load(path)
            .with_context(|| format!("failed to load WASM device '{}'", cfg.name))?;

        self.devices.push(Arc::new(device));
        Ok(())
    }
}

#[cfg(not(feature = "wasm"))]
impl DeviceRegistry {
    fn load_wasm_device(&mut self, cfg: &crate::conf::types::DeviceConfig) -> Result<()> {
        Err(anyhow!(
            "WASM device '{}' requested, but Snakeway was built without the `wasm` feature",
            cfg.name
        ))
    }
}
