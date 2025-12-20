use crate::config::{BuiltinDeviceKind, DeviceKind, SnakewayConfig};
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
    let raw = cfg.get("options").cloned().unwrap_or_else(|| cfg.clone());

    Ok(Arc::new(IdentityDevice::from_config(&raw)?))
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

    pub fn load_from_config(&mut self, config: &SnakewayConfig) -> Result<()> {
        let builders = builtin_builders();

        for cfg in &config.devices {
            if !cfg.enabled {
                continue;
            }

            match cfg.kind {
                DeviceKind::Wasm => {
                    self.load_wasm_device(cfg)?;
                }

                DeviceKind::Builtin => {
                    let kind = cfg
                        .builtin
                        .as_ref()
                        .ok_or_else(|| anyhow!("builtin device '{}' missing type", cfg.name))?;

                    let builder = builders
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

#[cfg(feature = "wasm")]
impl DeviceRegistry {
    fn load_wasm_device(&mut self, cfg: &crate::config::DeviceConfig) -> Result<()> {
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
    fn load_wasm_device(&mut self, cfg: &crate::config::DeviceConfig) -> Result<()> {
        Err(anyhow!(
            "WASM device '{}' requested, but Snakeway was built without the `wasm` feature",
            cfg.name
        ))
    }
}
