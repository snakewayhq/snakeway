use std::sync::Arc;

use super::Device;

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

    pub fn all(&self) -> &[Arc<dyn Device>] {
        &self.devices
    }
}
