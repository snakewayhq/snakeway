use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Default, Serialize, Clone)]
pub struct ConnectionFilterSpec {
    pub cidr_allow: Vec<String>,
    pub cidr_deny: Vec<String>,
    pub ip_family: IpFamilySpec,
    pub on_no_peer_addr: OnNoPeerAddrSpec,
}

#[derive(Debug, Deserialize, Default, Serialize, Clone)]
pub struct IpFamilySpec {
    pub ipv4: bool,
    pub ipv6: bool,
}

#[derive(Debug, Deserialize, Default, Serialize, Clone)]
pub enum OnNoPeerAddrSpec {
    #[default]
    Allow,
    Deny,
}
