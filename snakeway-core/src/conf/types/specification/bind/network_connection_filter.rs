use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Default, Serialize, Clone)]
pub(crate) struct NetworkConnectionFilterSpec {
    pub(crate) cidr: CidrSpec,
    pub(crate) ip_family: IpFamilySpec,
    pub(crate) on_no_peer_addr: OnNoPeerAddrSpec,
}

#[derive(Debug, Deserialize, Default, Serialize, Clone)]
pub(crate) struct CidrSpec {
    pub(crate) allow: Vec<String>,
    pub(crate) deny: Vec<String>,
}

#[derive(Debug, Deserialize, Default, Serialize, Clone)]
pub(crate) struct IpFamilySpec {
    pub(crate) ipv4: bool,
    pub(crate) ipv6: bool,
}

#[derive(Debug, Deserialize, Default, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub(crate) enum OnNoPeerAddrSpec {
    #[default]
    Allow,
    Deny,
}
