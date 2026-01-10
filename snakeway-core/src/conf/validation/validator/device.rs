use crate::conf::types::{DeviceConfig, Origin};
use crate::conf::validation::ValidationReport;
use ipnet::IpNet;
use std::net::IpAddr;

pub fn validate_devices(devices: &Vec<DeviceConfig>, report: &mut ValidationReport) {
    for device in devices {
        match device {
            DeviceConfig::Wasm(cfg) => {
                if !cfg.enable {
                    return;
                }

                if !cfg.path.is_file() {
                    report.error(
                        format!("invalid WASM device path : {}", cfg.path.display()),
                        device.origin(),
                        None,
                    );
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
                    report.error(
                        format!("invalid geo ip database path: {}", geoip_db.display()),
                        device.origin(),
                        None,
                    );
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
            report.error(format!("invalid trusted proxy: {}", proxy), origin, None);
        }
    }

    for network in networks {
        // Hard error: trust-all networks
        if network.prefix_len() == 0 {
            report.error(
                "trusted_proxies must not contain a catch-all network (0.0.0.0/0 or ::/0)"
                    .to_string(),
                origin,
                None,
            );
            continue;
        }

        // Trusting public IP ranges is a red flag.
        if !is_private_net(&network) {
            report.error(
                format!("trusted_proxies contains a public IP range: {network}"),
                origin,
                None,
            );
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
