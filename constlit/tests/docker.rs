use constlit::docker::{self, PortMapping, Protocol};
use std::net::{IpAddr, Ipv4Addr};

#[test]
fn port_accessors() {
    const P: PortMapping = docker::port!("127.0.0.1:8080:80/udp");
    assert_eq!(P.host_ip(), Some(IpAddr::V4(Ipv4Addr::LOCALHOST)));
    assert_eq!(P.host().map(|h| (h.start(), h.end())), Some((8080, 8080)));
    assert_eq!(P.container().start(), 80);
    assert_eq!(P.container().count(), 1);
    assert_eq!(P.protocol(), Protocol::Udp);
    assert_eq!(P.to_string(), "127.0.0.1:8080:80/udp");
    assert_eq!(docker::port!("80").to_string(), "80/tcp");
    assert_eq!(docker::port!("[::1]::80").to_string(), "[::1]::80/tcp");
    let range = docker::port!("8000-8010:8000-8010/sctp");
    assert_eq!(range.container().count(), 11);
    assert_eq!(range.to_string(), "8000-8010:8000-8010/sctp");
    assert_eq!(docker::port!("8080:80"), docker::port!("8080:80/tcp"));
    static S: PortMapping = docker::port!("443");
    assert_eq!(S.host(), None);
}
