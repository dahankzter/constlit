use constlit::{addr, ip};
use core::net::{Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6};

#[test]
fn net_types() {
    const A: Ipv4Addr = ip!("10.0.0.53");
    const B: Ipv6Addr = ip!("::1");
    const C: Ipv6Addr = ip!("2001:db8::ff00:42:8329");
    const S: SocketAddrV6 = addr!("[::1]:443");
    const T: SocketAddrV4 = addr!("0.0.0.0:8080");
    assert_eq!(A, "10.0.0.53".parse::<Ipv4Addr>().unwrap());
    assert_eq!(B, "::1".parse::<Ipv6Addr>().unwrap());
    assert_eq!(C, "2001:db8::ff00:42:8329".parse::<Ipv6Addr>().unwrap());
    assert_eq!(S, "[::1]:443".parse::<SocketAddrV6>().unwrap());
    assert_eq!(T, "0.0.0.0:8080".parse::<SocketAddrV4>().unwrap());
}
