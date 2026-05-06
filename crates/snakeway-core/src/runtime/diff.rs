use snakeway_conf::types::{ListenerConfig, RuntimeConfig};

#[derive(Debug, PartialEq)]
pub enum ConfigChangeKind {
    RuntimeOnly,
    ListenersChanged,
}

pub fn classify_config_change(old: &RuntimeConfig, new: &RuntimeConfig) -> ConfigChangeKind {
    if server_fields_changed(old, new) || listeners_changed(&old.listeners, &new.listeners) {
        ConfigChangeKind::ListenersChanged
    } else {
        ConfigChangeKind::RuntimeOnly
    }
}

fn server_fields_changed(old: &RuntimeConfig, new: &RuntimeConfig) -> bool {
    old.server.threads != new.server.threads || old.server.work_stealing != new.server.work_stealing
}

fn listeners_changed(old: &[ListenerConfig], new: &[ListenerConfig]) -> bool {
    if old.len() != new.len() {
        return true;
    }

    for (o, n) in old.iter().zip(new.iter()) {
        if !listener_equivalent(o, n) {
            return true;
        }
    }

    false
}

fn listener_equivalent(a: &ListenerConfig, b: &ListenerConfig) -> bool {
    a.name == b.name
        && a.addr == b.addr
        && a.tls_termination == b.tls_termination
        && a.enable_http2 == b.enable_http2
        && a.enable_admin == b.enable_admin
        && a.redirect == b.redirect
        && a.connection_filter == b.connection_filter
        && a.connection_rate_limiting_filter == b.connection_rate_limiting_filter
        && admin_auth_equivalent(&a.admin_auth, &b.admin_auth)
}

fn admin_auth_equivalent(
    a: &Option<snakeway_conf::types::AdminAuthConfig>,
    b: &Option<snakeway_conf::types::AdminAuthConfig>,
) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => {
            let a_file = a.bearer.as_ref().map(|b| &b.token_file);
            let b_file = b.bearer.as_ref().map(|b| &b.token_file);
            a_file == b_file
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use snakeway_conf::types::{ListenerConfig, RuntimeConfig, ServerConfig};
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn minimal_server() -> ServerConfig {
        ServerConfig {
            version: 1,
            threads: None,
            pid_file: PathBuf::new(),
            work_stealing: true,
            ca_file: None,
            tls_automation: None,
            observability: None,
            dns_refresh_interval_seconds: 30,
            upgrade_sock: None,
            upgrade_max_retries: None,
            grace_period_seconds: Some(10),
            graceful_shutdown_timeout_seconds: None,
        }
    }

    fn minimal_listener(name: &str, addr: &str) -> ListenerConfig {
        ListenerConfig {
            name: name.to_string(),
            addr: addr.to_string(),
            tls_termination: None,
            enable_http2: false,
            enable_admin: false,
            admin_auth: None,
            redirect: None,
            connection_filter: None,
            connection_rate_limiting_filter: None,
        }
    }

    fn minimal_config(listeners: Vec<ListenerConfig>) -> RuntimeConfig {
        RuntimeConfig {
            server: minimal_server(),
            listeners,
            routes: vec![],
            services: HashMap::new(),
            devices: vec![],
        }
    }

    #[test]
    fn identical_configs_are_runtime_only() {
        // Arrange
        let old = minimal_config(vec![minimal_listener("l0", "0.0.0.0:8080")]);
        let new = minimal_config(vec![minimal_listener("l0", "0.0.0.0:8080")]);

        // Act
        let kind = classify_config_change(&old, &new);

        // Assert
        assert_eq!(kind, ConfigChangeKind::RuntimeOnly);
    }

    #[test]
    fn listener_added() {
        // Arrange
        let old = minimal_config(vec![minimal_listener("l0", "0.0.0.0:8080")]);
        let new = minimal_config(vec![
            minimal_listener("l0", "0.0.0.0:8080"),
            minimal_listener("l1", "0.0.0.0:8443"),
        ]);

        // Act
        let kind = classify_config_change(&old, &new);

        // Assert
        assert_eq!(kind, ConfigChangeKind::ListenersChanged);
    }

    #[test]
    fn listener_removed() {
        // Arrange
        let old = minimal_config(vec![
            minimal_listener("l0", "0.0.0.0:8080"),
            minimal_listener("l1", "0.0.0.0:8443"),
        ]);
        let new = minimal_config(vec![minimal_listener("l0", "0.0.0.0:8080")]);

        // Act
        let kind = classify_config_change(&old, &new);

        // Assert
        assert_eq!(kind, ConfigChangeKind::ListenersChanged);
    }

    #[test]
    fn listener_addr_changed() {
        // Arrange
        let old = minimal_config(vec![minimal_listener("l0", "0.0.0.0:8080")]);
        let new = minimal_config(vec![minimal_listener("l0", "0.0.0.0:9090")]);

        // Act
        let kind = classify_config_change(&old, &new);

        // Assert
        assert_eq!(kind, ConfigChangeKind::ListenersChanged);
    }

    #[test]
    fn listener_http2_toggled() {
        // Arrange
        let old = minimal_config(vec![minimal_listener("l0", "0.0.0.0:8080")]);
        let mut new_listener = minimal_listener("l0", "0.0.0.0:8080");
        new_listener.enable_http2 = true;
        let new = minimal_config(vec![new_listener]);

        // Act
        let kind = classify_config_change(&old, &new);

        // Assert
        assert_eq!(kind, ConfigChangeKind::ListenersChanged);
    }

    #[test]
    fn threads_changed() {
        // Arrange
        let old = minimal_config(vec![minimal_listener("l0", "0.0.0.0:8080")]);
        let mut new = minimal_config(vec![minimal_listener("l0", "0.0.0.0:8080")]);
        new.server.threads = Some(16);

        // Act
        let kind = classify_config_change(&old, &new);

        // Assert
        assert_eq!(kind, ConfigChangeKind::ListenersChanged);
    }

    #[test]
    fn work_stealing_changed() {
        // Arrange
        let old = minimal_config(vec![minimal_listener("l0", "0.0.0.0:8080")]);
        let mut new = minimal_config(vec![minimal_listener("l0", "0.0.0.0:8080")]);
        new.server.work_stealing = false;

        // Act
        let kind = classify_config_change(&old, &new);

        // Assert
        assert_eq!(kind, ConfigChangeKind::ListenersChanged);
    }

    #[test]
    fn service_change_is_runtime_only() {
        // Arrange
        let old = minimal_config(vec![minimal_listener("l0", "0.0.0.0:8080")]);
        let mut new = minimal_config(vec![minimal_listener("l0", "0.0.0.0:8080")]);
        new.services.insert(
            "new-svc".to_string(),
            snakeway_conf::types::ServiceConfig {
                name: "new-svc".to_string(),
                listener: "l0".to_string(),
                load_balancing_strategy: snakeway_conf::types::LoadBalancingStrategy::RoundRobin,
                tcp_upstreams: vec![],
                unix_upstreams: vec![],
                circuit_breaker: Default::default(),
                health_check: Default::default(),
            },
        );

        // Act
        let kind = classify_config_change(&old, &new);

        // Assert
        assert_eq!(kind, ConfigChangeKind::RuntimeOnly);
    }
}
