use crate::conf::RuntimeConfig;
use crate::conf::types::DeviceConfig;
use crate::device::builtin::identity::IdentityDevice;
use crate::device::builtin::structured_logging::StructuredLoggingDevice;
use crate::device::core::Device;
#[cfg(feature = "wasm")]
use crate::device::wasm::wasm_device::WasmDevice;
use anyhow::Result;
use std::sync::Arc;

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
        for device_cfg in &cfg.devices {
            if !device_cfg.is_enabled() {
                continue;
            }

            match device_cfg {
                DeviceConfig::Identity(cfg) => {
                    let device_config = cfg.clone();
                    let device = Arc::new(IdentityDevice::from_config(device_config)?);
                    self.devices.push(device);
                }

                DeviceConfig::StructuredLogging(cfg) => {
                    let device_config = cfg.clone();
                    let device = Arc::new(StructuredLoggingDevice::from_config(device_config)?);
                    self.devices.push(device);
                }

                DeviceConfig::Wasm(cfg) => {
                    self.load_wasm_device(cfg)?;
                }
            }
        }

        Ok(())
    }

    pub fn all(&self) -> &[Arc<dyn Device>] {
        &self.devices
    }
}

impl DeviceRegistry {
    #[cfg(feature = "wasm")]
    fn load_wasm_device(&mut self, cfg: &crate::conf::types::WasmDeviceConfig) -> Result<()> {
        let device = WasmDevice::load(&cfg.path)?;

        self.devices.push(Arc::new(device));
        Ok(())
    }

    #[cfg(not(feature = "wasm"))]
    fn load_wasm_device(&mut self, cfg: &crate::conf::types::WasmDeviceConfig) -> Result<()> {
        Err(anyhow::anyhow!(
            "WASM device '{}' requested, but Snakeway was built without the `wasm` feature",
            cfg.path.display()
        ))
    }
}
