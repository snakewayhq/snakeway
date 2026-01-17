use crate::conf::types::{DeviceSpec, Origin};
use crate::conf::validation::ValidationReport;
use ipnet::IpNet;
use nix::NixPath;
use std::net::IpAddr;
use std::path::Path;

pub fn validate_devices(devices: &[DeviceSpec], report: &mut ValidationReport) {
    for device in devices {
        match device {
            DeviceSpec::Wasm(cfg) => {
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
            DeviceSpec::Identity(cfg) => {
                if !cfg.enable {
                    return;
                }

                validate_trusted_proxies(&cfg.trusted_proxies, report, device.origin());

                if cfg.enable_geoip {
                    if cfg.geoip_city_db.is_none()
                        && cfg.geoip_isp_db.is_none()
                        && cfg.geoip_connection_type_db.is_none()
                    {
                        report.geoip_enabled_with_no_dbs_specified(device.origin());
                    }

                    if let Some(path) = cfg.geoip_city_db.as_ref() {
                        validate_geoip_db_file(path, report, device.origin());
                    }

                    if let Some(path) = cfg.geoip_isp_db.as_ref() {
                        validate_geoip_db_file(path, report, device.origin());
                    }

                    if let Some(geoip_city_db) = cfg.geoip_connection_type_db.as_ref() {
                        validate_geoip_db_file(geoip_city_db, report, device.origin());
                    }
                }
            }
            DeviceSpec::StructuredLogging(cfg) => {
                if !cfg.enable {
                    return;
                }
            }
        };
    }
}

fn validate_geoip_db_file(geoip_db: &Path, report: &mut ValidationReport, origin: &Origin) -> bool {
    let mut has_error = false;
    if !geoip_db.is_file() {
        if geoip_db.is_empty() {
            report.geoip_db_path_is_empty(geoip_db.display(), origin);
            has_error = true;
        }
        if !geoip_db.exists() {
            report.geoip_db_path_does_not_exist(geoip_db.display(), origin);
            has_error = true;
        }
        if !geoip_db.is_file() {
            report.geoip_db_is_not_a_file(geoip_db.display(), origin);
            has_error = true;
        }
    }
    !has_error
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
        // Security note: Trusting all proxies is a catastrophic misconfiguration.
        if network.prefix_len() == 0 {
            report.trusted_proxies_cannot_trust_all_networks(origin);
        }

        // Trusting public IP ranges is a red flag/gray area.
        // Some environments must trust public IPs, but they should feel nervous about it.
        if !is_non_public_infra_network(&network) {
            report.trusted_proxies_contains_a_public_ip_range_warning(network, origin);
        }
    }
}

/// NOTE: This function identifies non-globally-routable infrastructure address
/// space (RFC1918, ULA, loopback, link-local).
/// It MUST NOT be used to determine the absolute trustworthiness of a peer.
fn is_non_public_infra_network(net: &IpNet) -> bool {
    match &net.addr() {
        IpAddr::V4(v4) => v4.is_private() || v4.is_loopback() || v4.is_link_local(),
        IpAddr::V6(v6) => v6.is_loopback() || v6.is_unique_local() || v6.is_unicast_link_local(),
    }
}
