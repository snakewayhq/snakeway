use crate::execution::device::builtin::identity::IdentityDevice;
use crate::execution::device::builtin::network_policy::NetworkPolicyDevice;
use crate::execution::device::builtin::request_filter::RequestFilterDevice;
use crate::execution::device::builtin::request_rate_limiting::RequestRateLimitingDevice;
use crate::execution::device::builtin::structured_logging::StructuredLoggingDevice;
use crate::execution::device::core::Device;
#[cfg(feature = "wasm")]
use crate::execution::device::wasm::{WasmDevice, create_wasm_engine};
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
        let mut wasm_configs: Vec<&snakeway_conf::types::WasmDeviceConfig> = Vec::new();

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

                // WASM devices are collected and loaded together so they share a single Engine.
                DeviceConfig::Wasm(cfg) => {
                    wasm_configs.push(cfg);
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

        self.load_wasm_devices(wasm_configs)?;

        Ok(())
    }

    pub(crate) fn all(&self) -> &[Arc<dyn Device>] {
        &self.devices
    }
}

impl DeviceRegistry {
    #[cfg(feature = "wasm")]
    fn load_wasm_devices(
        &mut self,
        wasm_configs: Vec<&snakeway_conf::types::WasmDeviceConfig>,
    ) -> Result<()> {
        if wasm_configs.is_empty() {
            return Ok(());
        }

        let engine = create_wasm_engine()?;

        for cfg in wasm_configs {
            let device = WasmDevice::load(
                Arc::clone(&engine),
                &cfg.path,
                cfg.name.clone(),
                cfg.fail_policy.clone(),
                cfg.timeout_ms,
                cfg.body_buffer_max,
                cfg.config.clone(),
            )?;
            info!("loaded wasm device: {}", device.name());
            self.devices.push(Arc::new(device));
        }

        Ok(())
    }

    #[cfg(not(feature = "wasm"))]
    fn load_wasm_devices(
        &mut self,
        wasm_configs: Vec<&snakeway_conf::types::WasmDeviceConfig>,
    ) -> Result<()> {
        if let Some(cfg) = wasm_configs.first() {
            return Err(anyhow::anyhow!(
                "WASM device '{}' requested, but Snakeway was built without the `wasm` feature",
                cfg.name
            ));
        }
        Ok(())
    }
}
