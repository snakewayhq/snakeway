use crate::conf::error::ConfigError;
use crate::conf::types::ServiceConfig;
use std::collections::HashMap;

pub fn merge_services(
    services: Vec<ServiceConfig>,
) -> Result<HashMap<String, ServiceConfig>, ConfigError> {
    let mut map = HashMap::new();

    for svc in services {
        if map.contains_key(&svc.name) {
            return Err(ConfigError::DuplicateService { name: svc.name });
        }
        map.insert(svc.name.clone(), svc);
    }

    Ok(map)
}
