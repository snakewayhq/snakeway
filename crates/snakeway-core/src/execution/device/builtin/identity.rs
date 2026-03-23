use crate::execution::ctx::{RequestCtx, ResponseCtx};
use crate::execution::device::core::errors::DeviceError;
use crate::execution::device::core::{Device, DeviceResult};
use crate::execution::enrichment::user_agent::{
    ClientIdentity, GeoInfo, UaEngine, build_ua_engine,
};
use crate::net::resolve_client_ip;
use ipnet::IpNet;
use maxminddb::PathElement;
use snakeway_conf::types::IdentityDeviceConfig;

pub(crate) struct IdentityDevice {
    // IP Trust
    trusted_proxies: Vec<IpNet>,
    max_x_forwarded_for_length: usize,

    // GeoIP
    pub(crate) enable_geoip: bool,
    city_reader: Option<maxminddb::Reader<maxminddb::Mmap>>,
    isp_reader: Option<maxminddb::Reader<maxminddb::Mmap>>,
    connection_type_reader: Option<maxminddb::Reader<maxminddb::Mmap>>,

    // User-agent
    pub(crate) enable_user_agent: bool,
    ua_engine: Option<UaEngine>,
    max_user_agent_length: usize,
}

impl IdentityDevice {
    pub(crate) fn from_config(cfg: IdentityDeviceConfig) -> anyhow::Result<Self> {
        // Safety note on these memory-mapped GeoIP files...
        // 1. File is opened read-only
        // 2. Lifetime is bound to IdentityDevice
        // 3. Snakeway does not mutate the mmdb file
        let geoip_city_db = match (cfg.enable_geoip, &cfg.geoip_city_db) {
            (true, Some(path)) => Some(unsafe { maxminddb::Reader::open_mmap(path)? }),
            _ => None,
        };

        let geoip_isp_db = match (cfg.enable_geoip, &cfg.geoip_isp_db) {
            (true, Some(path)) => Some(unsafe { maxminddb::Reader::open_mmap(path)? }),
            _ => None,
        };
        let geoip_connection_type_db = match (cfg.enable_geoip, &cfg.geoip_connection_type_db) {
            (true, Some(path)) => Some(unsafe { maxminddb::Reader::open_mmap(path)? }),
            _ => None,
        };

        let ua_engine = if cfg.enable_user_agent {
            Some(build_ua_engine(
                cfg.ua_engine,
                cfg.ua_parser_regexes.as_deref(),
            )?)
        } else {
            None
        };

        let trusted_proxies = cfg
            .trusted_proxies
            .iter()
            .map(|s| s.parse::<IpNet>())
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            // IP
            trusted_proxies,
            max_x_forwarded_for_length: cfg.max_x_forwarded_for_length,
            // GeoIP
            enable_geoip: cfg.enable_geoip,
            city_reader: geoip_city_db,
            isp_reader: geoip_isp_db,
            connection_type_reader: geoip_connection_type_db,
            // User-agent
            enable_user_agent: cfg.enable_user_agent,
            ua_engine,
            max_user_agent_length: cfg.max_user_agent_length,
        })
    }
}

impl Device for IdentityDevice {
    fn name(&self) -> &str {
        "Identity"
    }

    fn on_request(&self, ctx: &mut RequestCtx) -> DeviceResult {
        let (client_ip, proxy_chain, is_forwarded, is_trusted) = resolve_client_ip(
            ctx.headers(),
            ctx.peer_ip,
            &self.trusted_proxies,
            self.max_x_forwarded_for_length,
        );

        let mut identity = ClientIdentity {
            ip: client_ip,
            proxy_chain,
            is_forwarded,
            is_trusted,
            geo: None,
            ua: None,
        };

        if self.enable_geoip {
            let mut geo = GeoInfo::default();

            //-----------------------------------------------------------------
            // Country and Region
            //-----------------------------------------------------------------
            let lookup = self
                .city_reader
                .as_ref()
                .and_then(|reader| reader.lookup(client_ip).ok());

            if let Some(lookup) = lookup {
                geo.country_code = lookup
                    .decode_path::<String>(&[
                        PathElement::Key("country"),
                        PathElement::Key("iso_code"),
                    ])
                    .ok()
                    .flatten();

                geo.region = lookup
                    .decode_path::<String>(&[
                        PathElement::Key("subdivisions"),
                        PathElement::Index(0),
                        PathElement::Key("iso_code"),
                    ])
                    .ok()
                    .flatten();
            }

            //-----------------------------------------------------------------
            // ASN
            //-----------------------------------------------------------------
            let lookup = self
                .isp_reader
                .as_ref()
                .and_then(|reader| reader.lookup(client_ip).ok());

            if let Some(lookup) = lookup {
                geo.asn = lookup
                    .decode_path::<u32>(&[PathElement::Key("autonomous_system_number")])
                    .ok()
                    .flatten();

                geo.aso = lookup
                    .decode_path::<String>(&[PathElement::Key("autonomous_system_organization")])
                    .ok()
                    .flatten();
            }

            //-----------------------------------------------------------------
            // Connection-type
            //-----------------------------------------------------------------
            let lookup = self
                .connection_type_reader
                .as_ref()
                .and_then(|reader| reader.lookup(client_ip).ok());

            if let Some(lookup) = lookup {
                geo.connection_type = lookup
                    .decode_path::<String>(&[PathElement::Key("connection_type")])
                    .ok()
                    .flatten();
            }

            // Put it together...
            if geo.has_some_info() {
                identity.geo = Some(geo);
            }
        }

        if self.enable_user_agent {
            // User-Agent parsing
            if let Some((engine, ua)) = self.ua_engine.as_ref().zip(
                ctx.headers()
                    .get("user-agent")
                    .and_then(|v| v.to_str().ok())
                    .filter(|ua| ua.len() <= self.max_user_agent_length),
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
