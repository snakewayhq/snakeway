use ipnet::IpNet;
use ipnet_trie::IpnetTrie;
use std::fmt::Debug;
use std::net::IpAddr;

#[derive(Default, Clone)]
pub(crate) struct CidrCollection {
    table: IpnetTrie<()>,
}

impl CidrCollection {
    pub(crate) fn new(net_list: &[IpNet]) -> Self {
        let mut collection = IpnetTrie::new();
        for net in net_list {
            collection.insert(net.to_owned(), ());
        }

        Self { table: collection }
    }

    pub(crate) fn contains(&self, addr: IpAddr) -> bool {
        let host_net = IpNet::from(addr);
        self.table.longest_match(&host_net).is_some()
    }

    /// Returns the number of IPv4 and IPv6 networks in the collection.
    pub(crate) fn network_counts(&self) -> (usize, usize) {
        self.table.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.table.is_empty()
    }
}

impl Debug for CidrCollection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (ipv4, ipv6) = self.network_counts();
        f.debug_struct("CidrCollection")
            .field("cidr_collection_ipv4_count", &ipv4)
            .field("cidr_collection_ipv6_count", &ipv6)
            .finish()
    }
}

impl From<Vec<IpNet>> for CidrCollection {
    fn from(value: Vec<IpNet>) -> Self {
        Self::new(&value)
    }
}
