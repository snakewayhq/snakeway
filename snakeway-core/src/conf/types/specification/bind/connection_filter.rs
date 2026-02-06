use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Default, Serialize, Clone)]
pub struct ConnectionFilterSpec {
    pub cidr: CidrSpec,
    pub ip_family: IpFamilySpec,
    pub on_no_peer_addr: OnNoPeerAddrSpec,
}

#[derive(Debug, Deserialize, Default, Serialize, Clone)]
pub struct CidrSpec {
    pub allow: Vec<String>,
    pub deny: Vec<String>,
}

#[derive(Debug, Deserialize, Default, Serialize, Clone)]
pub struct IpFamilySpec {
    pub ipv4: bool,
    pub ipv6: bool,
}

#[derive(Debug, Deserialize, Default, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum OnNoPeerAddrSpec {
    #[default]
    Allow,
    Deny,
}
