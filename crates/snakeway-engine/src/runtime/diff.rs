use pingora::server::configuration::ServerConf;
use snakeway_conf::types::{ListenerConfig, RuntimeConfig};

#[derive(Debug, PartialEq)]
pub enum ConfigChangeKind {
    RuntimeOnly,
    UpgradeRequired,
    /// A zero-drop upgrade cannot apply these settings, so the reload is rejected.
    RestartRequired {
        settings: Vec<&'static str>,
    },
}

pub fn classify_config_change(old: &RuntimeConfig, new: &RuntimeConfig) -> ConfigChangeKind {
    let settings = restart_settings_changed(old, new);
    if !settings.is_empty() {
        return ConfigChangeKind::RestartRequired { settings };
    }

    if server_fields_changed(old, new) || listeners_changed(&old.listeners, &new.listeners) {
        ConfigChangeKind::UpgradeRequired
    } else {
        ConfigChangeKind::RuntimeOnly
    }
}

/// Why a reload is rejected.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RejectReason {
    /// A setting changed that even a zero-drop upgrade cannot apply.
    RestartRequired,
    /// The change needs a zero-drop upgrade, and this platform cannot perform one.
    UpgradeUnsupported,
}

impl RejectReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RestartRequired => "restart_required",
            Self::UpgradeUnsupported => "upgrade_unsupported",
        }
    }
}

/// What the reload loop does with a classified change.
#[derive(Debug, PartialEq)]
pub enum ReloadPlan {
    ApplyInPlace,
    Upgrade,
    Reject {
        reason: RejectReason,
        settings: Vec<&'static str>,
    },
}

/// Decide how to handle a classified change.
///
/// Zero-drop upgrades work only on Linux. On any other platform, pass `upgrade_supported` as
/// `false`, and a change that needs an upgrade is rejected instead of stopping the process.
pub fn plan_reload(change: ConfigChangeKind, upgrade_supported: bool) -> ReloadPlan {
    match change {
        ConfigChangeKind::RuntimeOnly => ReloadPlan::ApplyInPlace,
        ConfigChangeKind::UpgradeRequired if upgrade_supported => ReloadPlan::Upgrade,
        ConfigChangeKind::UpgradeRequired => ReloadPlan::Reject {
            reason: RejectReason::UpgradeUnsupported,
            settings: Vec::new(),
        },
        ConfigChangeKind::RestartRequired { settings } => ReloadPlan::Reject {
            reason: RejectReason::RestartRequired,
            settings,
        },
    }
}

/// The running process uses these settings to hand its listeners to a new process, so an
/// upgrade cannot apply a change to them.
fn restart_settings_changed(old: &RuntimeConfig, new: &RuntimeConfig) -> Vec<&'static str> {
    let mut settings = Vec::new();
    if old.server.pid_file != new.server.pid_file {
        settings.push("server.pid_file");
    }
    if effective_upgrade_sock(old) != effective_upgrade_sock(new) {
        settings.push("server.upgrade.sock");
    }
    settings
}

/// Pingora uses its own default path when `upgrade.sock` is not set, so an explicit default is
/// not a change.
fn effective_upgrade_sock(config: &RuntimeConfig) -> String {
    config
        .server
        .upgrade
        .sock
        .clone()
        .unwrap_or_else(|| ServerConf::default().upgrade_sock)
}

/// Settings that Pingora, the traffic proxy, or the control plane read only when the process
/// starts, so a change needs a new process.
fn server_fields_changed(old: &RuntimeConfig, new: &RuntimeConfig) -> bool {
    let (old, new) = (&old.server, &new.server);
    old.threads != new.threads
        || old.performance != new.performance
        || old.shutdown != new.shutdown
        || old.upstream != new.upstream
        || old.ca_file != new.ca_file
        || old.dns_refresh_interval_seconds != new.dns_refresh_interval_seconds
        || old.observability != new.observability
        || old.tls_automation != new.tls_automation
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
        && a.http2 == b.http2
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
    use snakeway_conf::types::{
        AcmeServerConfig, CertStoreConfig, ListenerConfig, ObservabilityConfig, PerformanceConfig,
        RuntimeConfig, ServerConfig, ShutdownConfig, TlsAutomationConfig, UpgradeConfig,
        UpstreamSettingsConfig, UpstreamSourceAddressesConfig, WasmConfig,
    };
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::time::Duration;

    fn minimal_server() -> ServerConfig {
        ServerConfig {
            version: 1,
            threads: None,
            pid_file: PathBuf::new(),
            ca_file: None,
            tls_automation: None,
            observability: None,
            dns_refresh_interval_seconds: 30,
            shutdown: ShutdownConfig {
                drain_seconds: Some(10),
                force_timeout_seconds: None,
            },
            upgrade: UpgradeConfig {
                sock: None,
                max_retries: None,
            },
            performance: PerformanceConfig {
                work_stealing: true,
                parallel_accepts_per_listener: None,
            },
            upstream: UpstreamSettingsConfig {
                connection_pool_size: None,
                connection_timeout: None,
                read_timeout: None,
                source_addresses: None,
            },
            wasm: WasmConfig::default(),
        }
    }

    fn minimal_listener(name: &str, addr: &str) -> ListenerConfig {
        ListenerConfig {
            name: name.to_string(),
            addr: addr.to_string(),
            tls_termination: None,
            enable_http2: false,
            http2: None,
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
        assert_eq!(kind, ConfigChangeKind::UpgradeRequired);
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
        assert_eq!(kind, ConfigChangeKind::UpgradeRequired);
    }

    #[test]
    fn listener_addr_changed() {
        // Arrange
        let old = minimal_config(vec![minimal_listener("l0", "0.0.0.0:8080")]);
        let new = minimal_config(vec![minimal_listener("l0", "0.0.0.0:9090")]);

        // Act
        let kind = classify_config_change(&old, &new);

        // Assert
        assert_eq!(kind, ConfigChangeKind::UpgradeRequired);
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
        assert_eq!(kind, ConfigChangeKind::UpgradeRequired);
    }

    #[test]
    fn listener_http2_options_changed() {
        // Arrange
        let make_tuned_listener = |max_concurrent_streams| {
            let mut listener = minimal_listener("l0", "0.0.0.0:8080");
            listener.enable_http2 = true;
            listener.http2 = Some(snakeway_conf::types::Http2Config {
                max_concurrent_streams: Some(max_concurrent_streams),
                max_header_list_size: None,
                initial_window_size: None,
                initial_connection_window_size: None,
            });
            listener
        };
        let old = minimal_config(vec![make_tuned_listener(100)]);
        let new = minimal_config(vec![make_tuned_listener(200)]);

        // Act
        let kind = classify_config_change(&old, &new);

        // Assert
        assert_eq!(kind, ConfigChangeKind::UpgradeRequired);
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
        assert_eq!(kind, ConfigChangeKind::UpgradeRequired);
    }

    #[test]
    fn work_stealing_changed() {
        // Arrange
        let old = minimal_config(vec![minimal_listener("l0", "0.0.0.0:8080")]);
        let mut new = minimal_config(vec![minimal_listener("l0", "0.0.0.0:8080")]);
        new.server.performance.work_stealing = false;

        // Act
        let kind = classify_config_change(&old, &new);

        // Assert
        assert_eq!(kind, ConfigChangeKind::UpgradeRequired);
    }

    /// An old config and a new config that differ only by the given server setting change.
    fn configs_with_server_change(
        change: impl FnOnce(&mut ServerConfig),
    ) -> (RuntimeConfig, RuntimeConfig) {
        let old = minimal_config(vec![minimal_listener("l0", "0.0.0.0:8080")]);
        let mut new = minimal_config(vec![minimal_listener("l0", "0.0.0.0:8080")]);
        change(&mut new.server);
        (old, new)
    }

    #[test]
    fn shutdown_drain_seconds_changed() {
        // Arrange
        let (old, new) = configs_with_server_change(|s| {
            s.shutdown.drain_seconds = Some(99);
        });

        // Act
        let kind = classify_config_change(&old, &new);

        // Assert
        assert_eq!(kind, ConfigChangeKind::UpgradeRequired);
    }

    #[test]
    fn shutdown_force_timeout_seconds_changed() {
        // Arrange
        let (old, new) = configs_with_server_change(|s| {
            s.shutdown.force_timeout_seconds = Some(60);
        });

        // Act
        let kind = classify_config_change(&old, &new);

        // Assert
        assert_eq!(kind, ConfigChangeKind::UpgradeRequired);
    }

    #[test]
    fn upstream_connection_pool_size_changed() {
        // Arrange
        let (old, new) = configs_with_server_change(|s| {
            s.upstream.connection_pool_size = Some(64);
        });

        // Act
        let kind = classify_config_change(&old, &new);

        // Assert
        assert_eq!(kind, ConfigChangeKind::UpgradeRequired);
    }

    #[test]
    fn upstream_source_addresses_changed() {
        // Arrange
        let (old, new) = configs_with_server_change(|s| {
            s.upstream.source_addresses = Some(UpstreamSourceAddressesConfig {
                ipv4: vec!["10.0.0.1".to_string()],
                ipv6: vec![],
            });
        });

        // Act
        let kind = classify_config_change(&old, &new);

        // Assert
        assert_eq!(kind, ConfigChangeKind::UpgradeRequired);
    }

    #[test]
    fn parallel_accepts_per_listener_changed() {
        // Arrange
        let (old, new) = configs_with_server_change(|s| {
            s.performance.parallel_accepts_per_listener = Some(4);
        });

        // Act
        let kind = classify_config_change(&old, &new);

        // Assert
        assert_eq!(kind, ConfigChangeKind::UpgradeRequired);
    }

    #[test]
    fn upstream_connection_timeout_changed() {
        // Arrange
        let (old, new) = configs_with_server_change(|s| {
            s.upstream.connection_timeout = Some(Duration::from_secs(5));
        });

        // Act
        let kind = classify_config_change(&old, &new);

        // Assert
        assert_eq!(kind, ConfigChangeKind::UpgradeRequired);
    }

    #[test]
    fn upstream_read_timeout_changed() {
        // Arrange
        let (old, new) = configs_with_server_change(|s| {
            s.upstream.read_timeout = Some(Duration::from_secs(30));
        });

        // Act
        let kind = classify_config_change(&old, &new);

        // Assert
        assert_eq!(kind, ConfigChangeKind::UpgradeRequired);
    }

    #[test]
    fn ca_file_changed() {
        // Arrange
        let (old, new) = configs_with_server_change(|s| {
            s.ca_file = Some("/etc/ssl/ca.pem".to_string());
        });

        // Act
        let kind = classify_config_change(&old, &new);

        // Assert
        assert_eq!(kind, ConfigChangeKind::UpgradeRequired);
    }

    #[test]
    fn dns_refresh_interval_changed() {
        // Arrange
        let (old, new) = configs_with_server_change(|s| {
            s.dns_refresh_interval_seconds = 60;
        });

        // Act
        let kind = classify_config_change(&old, &new);

        // Assert
        assert_eq!(kind, ConfigChangeKind::UpgradeRequired);
    }

    #[test]
    fn observability_changed() {
        // Arrange
        let (old, new) = configs_with_server_change(|s| {
            s.observability = Some(ObservabilityConfig::default());
        });

        // Act
        let kind = classify_config_change(&old, &new);

        // Assert
        assert_eq!(kind, ConfigChangeKind::UpgradeRequired);
    }

    #[test]
    fn tls_automation_changed() {
        // Arrange
        let (old, new) = configs_with_server_change(|s| {
            s.tls_automation = Some(TlsAutomationConfig {
                acme: AcmeServerConfig {
                    directory_url: "https://acme.example/dir".to_string(),
                    data_dir: PathBuf::from("/var/lib/snakeway/acme"),
                    contact_email: vec!["ops@example.com".to_string()],
                    ca_file: None,
                },
                cert_store: CertStoreConfig::Memory,
                renew_within_days: 30,
            });
        });

        // Act
        let kind = classify_config_change(&old, &new);

        // Assert
        assert_eq!(kind, ConfigChangeKind::UpgradeRequired);
    }

    #[test]
    fn upgrade_max_retries_change_is_runtime_only() {
        // Arrange
        let (old, new) = configs_with_server_change(|s| {
            s.upgrade.max_retries = Some(5);
        });

        // Act
        let kind = classify_config_change(&old, &new);

        // Assert
        assert_eq!(kind, ConfigChangeKind::RuntimeOnly);
    }

    #[test]
    fn pid_file_changed_requires_restart() {
        // Arrange
        let (old, new) = configs_with_server_change(|s| {
            s.pid_file = PathBuf::from("/run/snakeway-new.pid");
        });

        // Act
        let kind = classify_config_change(&old, &new);

        // Assert
        assert_eq!(
            kind,
            ConfigChangeKind::RestartRequired {
                settings: vec!["server.pid_file"]
            }
        );
    }

    #[test]
    fn upgrade_sock_changed_requires_restart() {
        // Arrange
        let (old, new) = configs_with_server_change(|s| {
            s.upgrade.sock = Some("/run/snakeway-upgrade.sock".to_string());
        });

        // Act
        let kind = classify_config_change(&old, &new);

        // Assert
        assert_eq!(
            kind,
            ConfigChangeKind::RestartRequired {
                settings: vec!["server.upgrade.sock"]
            }
        );
    }

    #[test]
    fn restart_setting_wins_over_upgrade_setting() {
        // Arrange
        let (old, new) = configs_with_server_change(|s| {
            s.pid_file = PathBuf::from("/run/snakeway-new.pid");
            s.threads = Some(8);
        });

        // Act
        let kind = classify_config_change(&old, &new);

        // Assert
        assert_eq!(
            kind,
            ConfigChangeKind::RestartRequired {
                settings: vec!["server.pid_file"]
            }
        );
    }

    #[test]
    fn every_changed_restart_setting_is_named() {
        // Arrange
        let (old, new) = configs_with_server_change(|s| {
            s.pid_file = PathBuf::from("/run/snakeway-new.pid");
            s.upgrade.sock = Some("/run/snakeway-upgrade.sock".to_string());
        });

        // Act
        let kind = classify_config_change(&old, &new);

        // Assert
        assert_eq!(
            kind,
            ConfigChangeKind::RestartRequired {
                settings: vec!["server.pid_file", "server.upgrade.sock"]
            }
        );
    }

    #[test]
    fn writing_the_default_upgrade_sock_is_runtime_only() {
        // Arrange
        let (old, new) = configs_with_server_change(|s| {
            s.upgrade.sock = Some("/tmp/pingora_upgrade.sock".to_string());
        });

        // Act
        let kind = classify_config_change(&old, &new);

        // Assert
        assert_eq!(kind, ConfigChangeKind::RuntimeOnly);
    }

    #[test]
    fn runtime_only_change_is_applied_in_place() {
        // Arrange
        let change = ConfigChangeKind::RuntimeOnly;

        // Act
        let plan = plan_reload(change, false);

        // Assert
        assert_eq!(plan, ReloadPlan::ApplyInPlace);
    }

    #[test]
    fn upgrade_required_change_upgrades_where_upgrades_are_supported() {
        // Arrange
        let change = ConfigChangeKind::UpgradeRequired;

        // Act
        let plan = plan_reload(change, true);

        // Assert
        assert_eq!(plan, ReloadPlan::Upgrade);
    }

    #[test]
    fn upgrade_required_change_is_rejected_where_upgrades_are_unsupported() {
        // Arrange
        let change = ConfigChangeKind::UpgradeRequired;

        // Act
        let plan = plan_reload(change, false);

        // Assert
        assert_eq!(
            plan,
            ReloadPlan::Reject {
                reason: RejectReason::UpgradeUnsupported,
                settings: vec![]
            }
        );
    }

    #[test]
    fn restart_required_change_is_rejected_on_every_platform() {
        // Arrange
        let change = ConfigChangeKind::RestartRequired {
            settings: vec!["server.pid_file"],
        };

        // Act
        let plan = plan_reload(change, true);

        // Assert
        assert_eq!(
            plan,
            ReloadPlan::Reject {
                reason: RejectReason::RestartRequired,
                settings: vec!["server.pid_file"]
            }
        );
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
