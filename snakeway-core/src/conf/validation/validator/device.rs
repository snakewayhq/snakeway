use crate::conf::types::DeviceConfig;
use crate::conf::validation::error::ConfigError;
use crate::conf::validation::validation_ctx::ValidationCtx;
use ipnet::IpNet;
use std::net::IpAddr;

pub fn validate_devices(devices: &Vec<DeviceConfig>, ctx: &mut ValidationCtx) {
    for device in devices {
        match device {
            DeviceConfig::Wasm(cfg) => {
                if !cfg.enable {
                    return;
                }

                if !cfg.path.is_file() {
                    ctx.push(ConfigError::InvalidWasmDevicePath {
                        path: cfg.path.clone(),
                    });
                }
            }
            DeviceConfig::Identity(cfg) => {
                if !cfg.enable {
                    return;
                }

                validate_trusted_proxies(&cfg.trusted_proxies, ctx);

                if cfg.enable
                    && cfg.enable_geoip
                    && let Some(geoip_db) = cfg.geoip_db.as_ref()
                    && !geoip_db.is_file()
                {
                    ctx.push(ConfigError::InvalidGeoIPDatabasePath {
                        path: geoip_db.clone(),
                    });
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

fn validate_trusted_proxies(proxies: &[String], ctx: &mut ValidationCtx) {
    let mut networks = Vec::new();
    for proxy in proxies {
        if let Ok(net) = proxy.parse::<IpNet>() {
            networks.push(net);
        } else {
            ctx.push(ConfigError::InvalidTrustedProxy {
                proxy: proxy.clone(),
            });
        }
    }

    for network in networks {
        // Hard error: trust-all networks
        if network.prefix_len() == 0 {
            ctx.push(ConfigError::InvalidTrustedProxyNetwork {
                reason: "trusted_proxies must not contain a catch-all network (0.0.0.0/0 or ::/0)"
                    .into(),
            });
            continue;
        }

        // Trusting public IP ranges is a red flag.
        if !is_private_net(&network) {
            ctx.push(ConfigError::SuspiciousTrustedProxy {
                network: network.to_string(),
                reason: "public IP ranges should only be trusted if they belong to known infrastructure (e.g. CDN or load balancer)"
                    .into(),
            });
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
