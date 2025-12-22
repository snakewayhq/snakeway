use crate::config::device::identity::IdentityConfig;
use crate::ctx::{RequestCtx, ResponseCtx};
use crate::device::core::errors::DeviceError;
use crate::device::core::{Device, DeviceResult};
use crate::enrichment::user_agent::{ClientIdentity, GeoInfo, UaEngine, build_ua_engine};
use anyhow::Context;
use http::HeaderMap;
use ipnet::IpNet;
use maxminddb::PathElement;
use std::net::IpAddr;

const MAX_USER_AGENT_LENGTH: usize = 2048;
const MAX_X_FORWARDED_FOR_LENGTH: usize = 1024;

pub struct IdentityDevice {
    trusted_proxies: Vec<IpNet>,
    geoip: Option<maxminddb::Reader<maxminddb::Mmap>>,
    ua_engine: Option<UaEngine>,
    pub enable_user_agent: bool,
    pub enable_geoip: bool,
}

impl IdentityDevice {
    pub fn from_config(raw: &toml::Value) -> anyhow::Result<Self> {
        let cfg: IdentityConfig = raw.clone().try_into().context("invalid identity config")?;

        let geoip = match (cfg.enable_geoip, &cfg.geoip_db) {
            (true, Some(path)) => {
                // SAFETY:
                // - File is opened read-only
                // - Lifetime is bound to IdentityDevice
                // - Snakeway does not mutate the mmdb file
                Some(unsafe { maxminddb::Reader::open_mmap(path)? })
            }
            (true, None) => {
                anyhow::bail!("enable_geoip=true but geoip_db is not set");
            }
            _ => None,
        };

        let ua_engine = if cfg.enable_user_agent {
            Some(build_ua_engine(cfg.ua_engine)?)
        } else {
            None
        };

        let trusted_proxies = cfg
            .trusted_proxies
            .iter()
            .map(|s| s.parse::<IpNet>())
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            geoip,
            ua_engine,
            trusted_proxies,
            enable_user_agent: cfg.enable_user_agent,
            enable_geoip: cfg.enable_geoip,
        })
    }
}

impl Device for IdentityDevice {
    fn on_request(&self, ctx: &mut RequestCtx) -> DeviceResult {
        let (client_ip, proxy_chain) =
            resolve_client_ip(&ctx.headers, ctx.peer_ip, &self.trusted_proxies);

        let mut identity = ClientIdentity {
            ip: client_ip,
            proxy_chain,
            geo: None,
            ua: None,
        };

        if self.enable_geoip {
            // GeoIP enrichment (country-only, EU-safe)
            let country_code: Option<String> = self
                .geoip
                .as_ref()
                .and_then(|reader| reader.lookup(client_ip).ok())
                .and_then(|lookup| {
                    lookup
                        .decode_path::<String>(&[
                            PathElement::Key("country"),
                            PathElement::Key("iso_code"),
                        ])
                        .ok()
                        .flatten()
                });

            if let Some(country_code) = country_code {
                identity.geo = Some(GeoInfo {
                    country_code: Some(country_code.to_ascii_uppercase()),
                    region: None,
                    asn: None,
                });
            }
        }

        if self.enable_user_agent {
            // User-Agent parsing
            if let Some((engine, ua)) = self.ua_engine.as_ref().zip(
                ctx.headers
                    .get("user-agent")
                    .and_then(|v| v.to_str().ok())
                    .filter(|ua| ua.len() <= MAX_USER_AGENT_LENGTH),
            ) {
                identity.ua = Some(engine.parse(ua));
            }
        }

        // Identity is authoritative and immutable after insertion.
        // Downstream devices MUST read from ctx.extensions and MUST NOT re-parse headers.
        ctx.extensions.insert(identity);
        DeviceResult::Continue
    }

    fn before_proxy(&self, _: &mut RequestCtx) -> DeviceResult {
        DeviceResult::Continue
    }

    fn after_proxy(&self, _: &mut ResponseCtx) -> DeviceResult {
        DeviceResult::Continue
    }

    fn on_response(&self, _: &mut ResponseCtx) -> DeviceResult {
        DeviceResult::Continue
    }

    fn on_error(&self, _: &DeviceError) {}
}

/// Resolve the true client IP using X-Forwarded-For and a trusted proxy list.
///
/// Returns:
/// - client_ip: the resolved client IP
/// - proxy_chain: ordered list of proxy IPs (closest first)
///
/// Rules:
/// - Walk XFF from right → left
/// - Stop at first IP not in trusted_proxies
/// - If no untrusted IP found, fall back to peer_ip
pub fn resolve_client_ip(
    headers: &HeaderMap,
    peer_ip: IpAddr,
    trusted_proxies: &[IpNet],
) -> (IpAddr, Vec<IpAddr>) {
    // If there are no trusted proxies, we can't trust XFF, so just return the peer IP.
    if trusted_proxies.is_empty() {
        return (peer_ip, Vec::new());
    }

    // Only trust XFF if the immediate peer is trusted
    if !trusted_proxies.iter().any(|net| net.contains(&peer_ip)) {
        return (peer_ip, Vec::new());
    }

    let xff = match headers.get("x-forwarded-for").and_then(|h| h.to_str().ok()) {
        Some(v) => v,
        None => return (peer_ip, Vec::new()),
    };

    // Guard against overly long XFF headers to prevent potential abuse.
    if xff.len() > MAX_X_FORWARDED_FOR_LENGTH {
        return (peer_ip, Vec::new());
    }

    let ips: Vec<IpAddr> = xff
        .split(',')
        .map(|s| s.trim())
        .filter_map(|s| s.parse::<IpAddr>().ok())
        .collect();

    let mut proxy_chain = Vec::with_capacity(ips.len());

    for ip in ips.iter().rev() {
        if trusted_proxies.iter().any(|net| net.contains(ip)) {
            proxy_chain.push(*ip);
            continue;
        }

        return (*ip, proxy_chain);
    }

    (peer_ip, proxy_chain)
}
