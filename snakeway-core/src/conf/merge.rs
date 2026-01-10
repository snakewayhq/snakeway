use crate::conf::types::{ListenerConfig, ServiceConfig};
use crate::conf::validation::ConfigError;
use std::collections::HashMap;

pub fn merge_services(
    services: Vec<ServiceConfig>,
) -> Result<HashMap<String, ServiceConfig>, ConfigError> {
    let mut map = HashMap::new();
    for svc in services {
        map.insert(svc.name.clone(), svc);
    }
    Ok(map)
}

#[derive(Hash, Eq, PartialEq)]
struct ListenerKey {
    addr: String,
    tls_cert: Option<String>,
    tls_key: Option<String>,
}

impl From<&ListenerConfig> for ListenerKey {
    fn from(l: &ListenerConfig) -> Self {
        let (tls_cert, tls_key) = match &l.tls {
            Some(tls) => (Some(tls.cert.clone()), Some(tls.key.clone())),
            None => (None, None),
        };

        Self {
            addr: l.addr.clone(),
            tls_cert,
            tls_key,
        }
    }
}

type MergedListeners = (Vec<ListenerConfig>, HashMap<String, String>);

pub fn merge_listeners(listeners: Vec<ListenerConfig>) -> Result<MergedListeners, ConfigError> {
    let mut merged: HashMap<ListenerKey, ListenerConfig> = HashMap::new();
    // old_name to canonical_name
    let mut name_map: HashMap<String, String> = HashMap::new();

    for l in listeners {
        let key = ListenerKey::from(&l);

        match merged.get_mut(&key) {
            Some(existing) => {
                // Merge flags
                existing.enable_http2 |= l.enable_http2;
                existing.enable_admin |= l.enable_admin;

                // Redirect must be unique
                if l.redirect.is_some() {
                    if existing.redirect.is_some() {
                        return Err(ConfigError::Custom {
                            message: format!("multiple redirects defined for {}", l.addr),
                        });
                    }
                    existing.redirect = l.redirect;
                }

                name_map.insert(l.name.clone(), existing.name.clone());
            }
            None => {
                let canonical_name = format!("listener@{}", l.addr);
                let mut new = l.clone();
                new.name = canonical_name.clone();

                name_map.insert(l.name.clone(), canonical_name.clone());
                merged.insert(key, new);
            }
        }
    }

    Ok((merged.values().cloned().collect(), name_map))
}
