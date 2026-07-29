//! The closed keyword vocabularies of the configuration language.
//!
//! Each `keyword_enum!` declares one set of words an operator may write for a
//! field, and generates the enum, its keyword list, and the conversions every
//! pipeline stage uses: spec validation checks a value against
//! `keyword_set()`, lowering narrows through the generated `TryFrom`, and the
//! runtime config holds the enum. The vocabulary belongs to none of those
//! stages, so it lives here and each stage imports it.

use serde::{Deserialize, Serialize};

confval::keyword_enum!(
    #[derive(Default, Deserialize, Serialize)]
    #[serde(rename_all = "lowercase")]
    pub WasmDeviceFailPolicy,
    {
        #[default]
        Open => "open",
        Closed => "closed",
    }
);

confval::keyword_enum!(
    #[derive(Default, Deserialize, Serialize)]
    #[serde(rename_all = "lowercase")]
    pub UaEngineKind,
    {
        UaParser => "uaparser",
        #[default]
        Woothee => "woothee",
    }
);

confval::keyword_enum!(
    #[derive(Deserialize, Serialize)]
    pub LoadBalancingStrategy,
    {
        Failover => "failover",
        RoundRobin => "round_robin",
        RequestPressure => "request_pressure",
        StickyHash => "sticky_hash",
        Random => "random",
    }
);

confval::keyword_enum!(
    #[derive(Default, Deserialize, Serialize)]
    #[serde(rename_all = "lowercase")]
    pub LogLevelConfig,
    {
        Trace => "trace",
        Debug => "debug",
        Info => "info",
        Warn => "warn",
        #[default]
        Error => "error",
    }
);

confval::keyword_enum!(
    #[derive(Deserialize, Serialize)]
    #[serde(rename_all = "snake_case")]
    pub LogEventConfig,
    {
        Request => "request",
        BeforeProxy => "before_proxy",
        AfterProxy => "after_proxy",
        Response => "response",
    }
);

confval::keyword_enum!(
    #[derive(Deserialize, Serialize)]
    #[serde(rename_all = "lowercase")]
    pub LogPhaseConfig,
    {
        Request => "request",
        Response => "response",
    }
);

confval::keyword_enum!(
    #[derive(Deserialize, Serialize)]
    #[serde(rename_all = "snake_case")]
    pub IdentityFieldConfig,
    {
        ClientIp => "client_ip",
        ProxyChain => "proxy_chain",
        Forwarded => "forwarded",
        Trusted => "trusted",
        Asn => "asn",
        Aso => "aso",
        Country => "country",
        Region => "region",
        ConnectionType => "connection_type",
        Bot => "bot",
        Device => "device",
    }
);

confval::keyword_enum!(
    #[derive(Default, Deserialize, Serialize)]
    #[serde(rename_all = "lowercase")]
    pub OnInvalidForwardedConfig,
    {
        Deny => "deny",
        #[default]
        Ignore => "ignore",
    }
);
