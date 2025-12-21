use crate::config::device::identity::IdentityConfig;
use crate::ctx::{RequestCtx, ResponseCtx};
use crate::device::core::errors::DeviceError;
use crate::device::core::{Device, DeviceResult};
use crate::enrichment::user_agent::{ClientIdentity, GeoInfo, UaEngine, build_ua_engine};
use ipnet::IpNet;

use crate::enrichment::geoip::resolve_client_ip;
use maxminddb::PathElement;

pub struct IdentityDevice {
    trusted_proxies: Vec<IpNet>,
    geoip: Option<maxminddb::Reader<maxminddb::Mmap>>,
    ua_engine: Option<UaEngine>,
}

impl IdentityDevice {
    pub fn from_config(raw: &toml::Value) -> anyhow::Result<Self> {
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

        let trusted_proxies = cfg
            .trusted_proxies
            .iter()
            .map(|s| s.parse::<IpNet>())
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            geoip,
            ua_engine,
            trusted_proxies,
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

        // GeoIP enrichment (country-only, EU-safe)
        let country_code = self
            .geoip
            .as_ref()
            .and_then(|reader| reader.lookup(client_ip).ok())
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
