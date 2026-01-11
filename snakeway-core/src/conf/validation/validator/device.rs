use crate::conf::types::{DeviceConfig, Origin};
use crate::conf::validation::ValidationReport;
use ipnet::IpNet;
use nix::NixPath;
use std::net::IpAddr;

pub fn validate_devices(devices: &[DeviceConfig], report: &mut ValidationReport) {
    for device in devices {
        match device {
            DeviceConfig::Wasm(cfg) => {
                if !cfg.enable {
                    return;
                }

                if cfg.path.is_empty() {
                    report.wasm_device_path_is_empty(cfg.path.display(), device.origin());
                }
                if !cfg.path.exists() {
                    report.wasm_device_path_does_not_exist(cfg.path.display(), device.origin());
                }
                if !cfg.path.is_file() {
                    report.wasm_device_path_is_not_a_file(cfg.path.display(), device.origin());
                }
            }
            DeviceConfig::Identity(cfg) => {
                if !cfg.enable {
                    return;
                }

                validate_trusted_proxies(&cfg.trusted_proxies, report, device.origin());

                if cfg.enable
                    && cfg.enable_geoip
                    && let Some(geoip_db) = cfg.geoip_db.as_ref()
                    && !geoip_db.is_file()
                {
                    if geoip_db.is_empty() {
                        report.geoip_db_path_is_empty(geoip_db.display(), device.origin());
                    }
                    if !geoip_db.exists() {
                        report.geoip_db_path_does_not_exist(geoip_db.display(), device.origin());
                    }
                    if !geoip_db.is_file() {
                        report.geoip_db_is_not_a_file(geoip_db.display(), device.origin());
                    }
                }
                if cfg.enable && cfg.enable_user_agent {
                    return;
                }
            }
            DeviceConfig::StructuredLogging(cfg) => {
                if !cfg.enable {
                    return;
                }
            }
        };
    }
}

fn validate_trusted_proxies(proxies: &[String], report: &mut ValidationReport, origin: &Origin) {
    let mut networks = Vec::new();
    for proxy in proxies {
        if let Ok(net) = proxy.parse::<IpNet>() {
            networks.push(net);
        } else {
            report.invalid_trusted_proxy(proxy, origin);
        }
    }

    for network in networks {
        // Hard error - trust-all networks
        if network.prefix_len() == 0 {
            report.trusted_proxies_cannot_trust_all_networks(origin);
        }

        // Trusting public IP ranges is a red flag.
        if !is_private_net(&network) {
            report.trusted_proxies_contains_a_public_ip_range(network, origin);
        }
    }
}

fn is_private_net(net: &IpNet) -> bool {
    is_private_ip(&net.addr())
}

fn is_private_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_private() || v4.is_loopback() || v4.is_link_local(),
        IpAddr::V6(v6) => v6.is_loopback() || v6.is_unique_local() || v6.is_unicast_link_local(),
    }
}
