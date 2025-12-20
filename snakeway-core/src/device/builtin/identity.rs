use crate::config::device::identity::IdentityConfig;
use crate::ctx::{RequestCtx, ResponseCtx};
use crate::device::core::errors::DeviceError;
use crate::device::core::{Device, DeviceResult};
use crate::user_agent::{ClientIdentity, GeoInfo, UaEngine, build_ua_engine};

use maxminddb::PathElement;
use std::net::IpAddr;

pub struct IdentityDevice {
    geoip: Option<maxminddb::Reader<maxminddb::Mmap>>,
    ua_engine: Option<UaEngine>,
}

impl IdentityDevice {
    pub fn from_config(raw: &toml::Value) -> anyhow::Result<Self> {
        tracing::info!("identity raw options = {}", raw.to_string());
        let cfg: IdentityConfig = raw.clone().try_into()?;

        let geoip = match (cfg.enable_geoip, &cfg.geoip_db) {
            (true, Some(path)) => {
                // SAFETY:
                // - File is opened read-only
                // - Lifetime is bound to IdentityDevice
                // - Snakeway does not mutate the mmdb file
                Some(unsafe { maxminddb::Reader::open_mmap(path)? })
            }
            _ => None,
        };

        let ua_engine = if cfg.enable_user_agent {
            Some(build_ua_engine(cfg.ua_engine)?)
        } else {
            None
        };

        Ok(Self { geoip, ua_engine })
    }
}

impl Device for IdentityDevice {
    fn on_request(&self, ctx: &mut RequestCtx) -> DeviceResult {
        // TEMPORARY: peer IP resolution placeholder
        // todo This MUST be replaced with a trusted-proxy-aware resolver later.
        let peer_ip = ctx
            .headers
            .get("x-forwarded-for")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.split(',').next())
            .and_then(|s| s.trim().parse::<IpAddr>().ok())
            .unwrap_or_else(|| "127.0.0.1".parse().unwrap());

        let mut identity = ClientIdentity {
            ip: peer_ip,
            proxy_chain: Vec::new(),
            geo: None,
            ua: None,
        };

        // GeoIP enrichment (country-only, EU-safe)

        let country_code = self
            .geoip
            .as_ref()
            .and_then(|reader| reader.lookup(peer_ip).ok())
            .and_then(|lookup| {
                lookup
                    .decode_path::<Option<String>>(&[
                        PathElement::Key("country"),
                        PathElement::Key("iso_code"),
                    ])
                    .ok()
                    .flatten()
            });

        if let Some(country_code) = country_code {
            identity.geo = Some(GeoInfo {
                country_code,
                region: None,
                asn: None,
            });
        }

        // User-Agent parsing
        if let Some(engine) = &self.ua_engine {
            if let Some(ua) = ctx.headers.get("user-agent").and_then(|v| v.to_str().ok()) {
                identity.ua = Some(engine.parse(ua));
            }
        }

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
