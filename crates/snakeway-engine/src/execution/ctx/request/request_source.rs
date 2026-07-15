use http::{HeaderMap, Method, Uri, Version};
use pingora::prelude::Session;
use pingora::protocols::Digest;
use pingora::protocols::l4::socket::SocketAddr as PingoraSocketAddr;
use std::net::{IpAddr, Ipv4Addr};

pub trait RequestSource {
    fn http_uri(&self) -> &Uri;
    fn http_method(&self) -> &Method;
    fn http_headers(&self) -> &HeaderMap;
    fn http_version(&self) -> Version;
    fn http_is_upgrade_req(&self) -> bool;
    fn net_peer_ip(&self) -> IpAddr;
    fn net_digest(&self) -> Option<&Digest>;
}

impl RequestSource for Session {
    fn http_uri(&self) -> &Uri {
        &self.req_header().uri
    }

    fn http_method(&self) -> &Method {
        &self.req_header().method
    }

    fn http_headers(&self) -> &HeaderMap {
        &self.req_header().headers
    }

    fn http_version(&self) -> Version {
        self.req_header().version
    }

    fn http_is_upgrade_req(&self) -> bool {
        self.is_upgrade_req()
    }

    fn net_peer_ip(&self) -> IpAddr {
        match self.client_addr() {
            Some(PingoraSocketAddr::Inet(addr)) => addr.ip(),
            _ => IpAddr::V4(Ipv4Addr::UNSPECIFIED),
        }
    }

    fn net_digest(&self) -> Option<&Digest> {
        self.digest()
    }
}
