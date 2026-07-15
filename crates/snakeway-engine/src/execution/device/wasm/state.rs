use super::bindings::snakeway::device::host;
use opentelemetry::KeyValue;
use opentelemetry::metrics::Counter;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use wasmtime::StoreLimits;

pub(crate) struct HostState {
    pub(crate) config: Arc<HashMap<String, String>>,
    pub(crate) device_name: Arc<str>,
    pub(crate) limits: StoreLimits,
    pub(crate) custom_metrics: Counter<u64>,
}

impl super::bindings::snakeway::device::types::Host for HostState {}

#[hotpath::measure_all]
impl host::Host for HostState {
    fn config_get(&mut self, key: String) -> Option<String> {
        self.config.get(&key).cloned()
    }

    fn log(&mut self, level: u8, message: String) {
        match level {
            0 => tracing::trace!(device = %self.device_name, "{message}"),
            1 => tracing::debug!(device = %self.device_name, "{message}"),
            2 => tracing::info!(device = %self.device_name, "{message}"),
            3 => tracing::warn!(device = %self.device_name, "{message}"),
            _ => tracing::error!(device = %self.device_name, "{message}"),
        }
    }

    fn metric_increment(&mut self, name: String, delta: u64) {
        self.custom_metrics.add(
            delta,
            &[
                KeyValue::new("device", self.device_name.to_string()),
                KeyValue::new("metric", name),
            ],
        );
    }

    fn epoch_secs(&mut self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}
