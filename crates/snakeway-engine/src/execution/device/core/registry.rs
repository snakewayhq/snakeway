use crate::execution::device::builtin::identity::IdentityDevice;
use crate::execution::device::builtin::network_policy::NetworkPolicyDevice;
use crate::execution::device::builtin::request_filter::RequestFilterDevice;
use crate::execution::device::builtin::request_rate_limiting::RequestRateLimitingDevice;
use crate::execution::device::builtin::structured_logging::StructuredLoggingDevice;
use crate::execution::device::core::Device;
use crate::execution::device::wasm::{WasmDevice, WasmEngine};
use anyhow::Result;
use snakeway_conf::types::{DeviceConfig, RuntimeConfig};
use std::sync::Arc;
use tracing::info;

pub struct DeviceRegistry {
    devices: Vec<Arc<dyn Device>>,
    _wasm_engine: Option<WasmEngine>,
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
            _wasm_engine: None,
        }
    }
}

/// The canonical pipeline order of the device kinds.
/// The declaration order is the execution order.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum PipelineSlot {
    RequestFilter,
    Identity,
    NetworkPolicy,
    RequestRateLimiting,
    Wasm,
    StructuredLogging,
}

/// A device loaded from config, tagged with its pipeline slot so a stable
/// sort can impose the canonical order.
struct RankedDevice {
    slot: PipelineSlot,
    device: Arc<dyn Device>,
}

impl DeviceRegistry {
    pub(crate) fn load_from_config(&mut self, cfg: &RuntimeConfig) -> Result<()> {
        let mut wasm_configs: Vec<&snakeway_conf::types::WasmDeviceConfig> = Vec::new();
        let mut loaded: Vec<RankedDevice> = Vec::with_capacity(cfg.devices.len());

        for device_cfg in &cfg.devices {
            if !device_cfg.is_enabled() {
                continue;
            }

            match device_cfg {
                DeviceConfig::RequestFilter(cfg) => {
                    let device_config = cfg.as_ref().clone();
                    let device = Arc::new(RequestFilterDevice::from(device_config));
                    info!("loaded device: {}", device.name());
                    loaded.push(RankedDevice {
                        slot: PipelineSlot::RequestFilter,
                        device,
                    });
                }

                DeviceConfig::Identity(cfg) => {
                    let device_config = cfg.clone();
                    let device = Arc::new(IdentityDevice::try_from(device_config)?);
                    info!("loaded device: {}", device.name());
                    loaded.push(RankedDevice {
                        slot: PipelineSlot::Identity,
                        device,
                    });
                }

                DeviceConfig::NetworkPolicy(cfg) => {
                    let device_config = cfg.clone();
                    let device: Arc<NetworkPolicyDevice> = Arc::new(device_config.into());
                    info!("loaded device: {}", device.name());
                    loaded.push(RankedDevice {
                        slot: PipelineSlot::NetworkPolicy,
                        device,
                    });
                }

                DeviceConfig::RequestRateLimiting(cfg) => {
                    let device_config = cfg.clone();
                    let device: Arc<RequestRateLimitingDevice> = Arc::new(device_config.into());
                    info!("loaded device: {}", device.name());
                    loaded.push(RankedDevice {
                        slot: PipelineSlot::RequestRateLimiting,
                        device,
                    });
                }

                // WASM devices are collected and loaded together so they share a single Engine.
                DeviceConfig::Wasm(cfg) => {
                    wasm_configs.push(cfg);
                }

                DeviceConfig::StructuredLogging(cfg) => {
                    let device_config = cfg.clone();
                    let device = Arc::new(StructuredLoggingDevice::from(device_config));
                    info!("loaded device: {}", device.name());
                    loaded.push(RankedDevice {
                        slot: PipelineSlot::StructuredLogging,
                        device,
                    });
                }
            }
        }

        for device in self.load_wasm_devices(wasm_configs, &cfg.server.wasm)? {
            loaded.push(RankedDevice {
                slot: PipelineSlot::Wasm,
                device,
            });
        }

        loaded.sort_by_key(|ranked| ranked.slot);
        self.devices = loaded.into_iter().map(|ranked| ranked.device).collect();

        Ok(())
    }

    pub fn all(&self) -> &[Arc<dyn Device>] {
        &self.devices
    }
}

impl DeviceRegistry {
    fn load_wasm_devices(
        &mut self,
        wasm_configs: Vec<&snakeway_conf::types::WasmDeviceConfig>,
        wasm_settings: &snakeway_conf::types::WasmConfig,
    ) -> Result<Vec<Arc<dyn Device>>> {
        if wasm_configs.is_empty() {
            return Ok(Vec::new());
        }

        let wasm_engine = WasmEngine::new(wasm_settings)?;

        let mut devices = Vec::new();
        for cfg in wasm_configs {
            let device = WasmDevice::load(
                Arc::clone(&wasm_engine.engine),
                cfg,
                wasm_settings.max_memory_bytes,
            )?;
            info!("loaded wasm device: {}", device.name());
            devices.push(Arc::new(device) as Arc<dyn Device>);
        }

        self._wasm_engine = Some(wasm_engine);
        Ok(devices)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use confval::prelude::Lower;
    use snakeway_conf::types::{
        DeviceConfig, IdentityDeviceConfig, NetworkPolicyDeviceConfig, RequestFilterDeviceConfig,
        RequestRateLimitingDeviceConfig, ServerConfig, ServerSpec, StructuredLoggingDeviceConfig,
    };
    use std::collections::HashMap;

    fn config_with_devices(devices: Vec<DeviceConfig>) -> RuntimeConfig {
        RuntimeConfig {
            server: ServerConfig::lower(
                &ServerSpec::default(),
                &mut confval::prelude::Report::new(),
            )
            .unwrap(),
            listeners: vec![],
            routes: vec![],
            services: HashMap::new(),
            devices,
        }
    }

    fn enabled_devices_in_reverse_pipeline_order() -> Vec<DeviceConfig> {
        vec![
            DeviceConfig::StructuredLogging(StructuredLoggingDeviceConfig {
                enable: true,
                ..Default::default()
            }),
            DeviceConfig::RequestRateLimiting(RequestRateLimitingDeviceConfig {
                enable: true,
                ..Default::default()
            }),
            DeviceConfig::NetworkPolicy(NetworkPolicyDeviceConfig {
                enable: true,
                ..Default::default()
            }),
            DeviceConfig::Identity(IdentityDeviceConfig {
                enable: true,
                ..Default::default()
            }),
            DeviceConfig::RequestFilter(Box::new(RequestFilterDeviceConfig {
                enable: true,
                ..Default::default()
            })),
        ]
    }

    #[test]
    fn devices_load_in_pipeline_order_regardless_of_config_order() {
        // Arrange
        let cfg = config_with_devices(enabled_devices_in_reverse_pipeline_order());
        let mut registry = DeviceRegistry::new();

        // Act
        registry.load_from_config(&cfg).unwrap();

        // Assert
        let names: Vec<&str> = registry.all().iter().map(|d| d.name()).collect();
        assert_eq!(
            names,
            vec![
                "Request Filter",
                "Identity",
                "Network Policy",
                "Request Rate Limit",
                "Structured Logging",
            ]
        );
    }

    #[test]
    fn devices_of_one_kind_keep_their_config_order() {
        // Arrange
        let cfg = config_with_devices(vec![
            DeviceConfig::NetworkPolicy(NetworkPolicyDeviceConfig {
                enable: true,
                ..Default::default()
            }),
            DeviceConfig::NetworkPolicy(NetworkPolicyDeviceConfig {
                enable: true,
                ..Default::default()
            }),
        ]);
        let mut registry = DeviceRegistry::new();

        // Act
        registry.load_from_config(&cfg).unwrap();

        // Assert
        assert_eq!(registry.all().len(), 2);
    }
}
