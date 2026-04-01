use crate::execution::device::builtin::identity::IdentityDevice;
use crate::execution::device::builtin::network_policy::NetworkPolicyDevice;
use crate::execution::device::builtin::request_filter::RequestFilterDevice;
use crate::execution::device::builtin::request_rate_limiting::RequestRateLimitingDevice;
use crate::execution::device::builtin::structured_logging::StructuredLoggingDevice;
use crate::execution::device::core::Device;
#[cfg(feature = "wasm")]
use crate::execution::device::wasm::wasm_device::WasmDevice;
use anyhow::Result;
use snakeway_conf::types::{DeviceConfig, RuntimeConfig};
use std::sync::Arc;
use tracing::info;

pub(crate) struct DeviceRegistry {
    devices: Vec<Arc<dyn Device>>,
}

impl Default for DeviceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceRegistry {
    pub(crate) fn new() -> Self {
        Self {
            devices: Vec::new(),
        }
    }

    pub(crate) fn load_from_config(&mut self, cfg: &RuntimeConfig) -> Result<()> {
        for device_cfg in &cfg.devices {
            if !device_cfg.is_enabled() {
                continue;
            }

            match device_cfg {
                // Stateless devices are run before stateful devices as they are cheaper to run.
                // The request filter device specifically must run before the identity device,
                // as this allows it to short-circuit the request early to avoid unnecessary allocations.
                DeviceConfig::RequestFilter(cfg) => {
                    let device_config = cfg.as_ref().clone();
                    let device = Arc::new(RequestFilterDevice::from(device_config));
                    info!("loaded device: {}", device.name());
                    self.devices.push(device);
                }

                // Important: The identity device must always be first AFTER stateless devices,
                // so that it can establish the context of the request BEFORE all other stateful devices run.
                DeviceConfig::Identity(cfg) => {
                    let device_config = cfg.clone();
                    let device = Arc::new(IdentityDevice::try_from(device_config)?);
                    info!("loaded device: {}", device.name());
                    self.devices.push(device);
                }

                // Network policy devices are run after the identity device, as they need the client IP.
                // If the client identity is not available, the NetworkPolicy is ignored.
                DeviceConfig::NetworkPolicy(cfg) => {
                    let device_config = cfg.clone();
                    let device: Arc<NetworkPolicyDevice> = Arc::new(device_config.into());
                    info!("loaded device: {}", device.name());
                    self.devices.push(device);
                }

                DeviceConfig::RequestRateLimiting(cfg) => {
                    let device_config = cfg.clone();
                    let device: Arc<RequestRateLimitingDevice> = Arc::new(device_config.into());
                    info!("loaded device: {}", device.name());
                    self.devices.push(device);
                }

                // Wasm devices are loaded dynamically at runtime.
                // They should be run AFTER all builtin devices, except the logging device.
                DeviceConfig::Wasm(cfg) => {
                    info!("loaded device: {}", cfg.path.display());
                    self.load_wasm_device(cfg)?;
                }

                // Important: The logging device must always be last, so that it can observe all
                // other devices' outputs.
                DeviceConfig::StructuredLogging(cfg) => {
                    let device_config = cfg.clone();
                    let device = Arc::new(StructuredLoggingDevice::from(device_config));
                    info!("loaded device: {}", device.name());
                    self.devices.push(device);
                }
            }
        }

        Ok(())
    }

    pub(crate) fn all(&self) -> &[Arc<dyn Device>] {
        &self.devices
    }
}

impl DeviceRegistry {
    #[cfg(feature = "wasm")]
    fn load_wasm_device(&mut self, cfg: &snakeway_conf::types::WasmDeviceConfig) -> Result<()> {
        let device = WasmDevice::load(&cfg.path)?;

        self.devices.push(Arc::new(device));
        Ok(())
    }

    #[cfg(not(feature = "wasm"))]
    fn load_wasm_device(&mut self, cfg: &snakeway_conf::types::WasmDeviceConfig) -> Result<()> {
        Err(anyhow::anyhow!(
            "WASM device '{}' requested, but Snakeway was built without the `wasm` feature",
            cfg.path.display()
        ))
    }
}
