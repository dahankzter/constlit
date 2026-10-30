use constlit::{Cidr, Ipv4Cidr, Ipv6Cidr, cidr};
use core::net::{Ipv4Addr, Ipv6Addr};
use ipnet::{Ipv4Net, Ipv6Net};

fn splitmix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

#[test]
fn cidr_in_const() {
    const NET: Ipv4Cidr = cidr!("10.0.0.0/8");
    const DOC: Ipv6Cidr = cidr!("2001:db8::/32");
    assert_eq!(NET.to_string(), "10.0.0.0/8");
    assert_eq!(DOC.to_string(), "2001:db8::/32");
    let either: Cidr = NET.into();
    assert!(either.contains("10.1.2.3".parse().unwrap()));
}

#[test]
fn cidr_oracle_v4() {
    let mut state = 1;
    for _ in 0..2000 {
        let prefix = (splitmix(&mut state) % 33) as u8;
        let addr = Ipv4Addr::from_bits(splitmix(&mut state) as u32);
        let oracle = Ipv4Net::new(addr, prefix).unwrap().trunc();
        assert_eq!(
            Ipv4Cidr::new(addr, prefix).is_ok(),
            oracle.addr() == addr,
            "{addr}/{prefix}"
        );
        let ours = Ipv4Cidr::new(oracle.network(), prefix).unwrap();
        assert_eq!(ours.network(), oracle.network());
        assert_eq!(ours.netmask(), oracle.netmask());
        assert_eq!(ours.broadcast(), oracle.broadcast());
        for probe in [
            addr,
            oracle.broadcast(),
            Ipv4Addr::from_bits(splitmix(&mut state) as u32),
        ] {
            assert_eq!(
                ours.contains(probe),
                oracle.contains(&probe),
                "{ours} {probe}"
            );
        }
    }
}

#[test]
fn cidr_oracle_v6() {
    let mut state = 2;
    for _ in 0..2000 {
        let prefix = (splitmix(&mut state) % 129) as u8;
        let bits = |s: &mut u64| (u128::from(splitmix(s)) << 64) | u128::from(splitmix(s));
        let addr = Ipv6Addr::from_bits(bits(&mut state));
        let oracle = Ipv6Net::new(addr, prefix).unwrap().trunc();
        assert_eq!(
            Ipv6Cidr::new(addr, prefix).is_ok(),
            oracle.addr() == addr,
            "{addr}/{prefix}"
        );
        let ours = Ipv6Cidr::new(oracle.network(), prefix).unwrap();
        assert_eq!(ours.network(), oracle.network());
        assert_eq!(ours.netmask(), oracle.netmask());
        for probe in [
            addr,
            oracle.broadcast(),
            Ipv6Addr::from_bits(bits(&mut state)),
        ] {
            assert_eq!(
                ours.contains(probe),
                oracle.contains(&probe),
                "{ours} {probe}"
            );
        }
    }
}

#[cfg(feature = "ipnet")]
#[test]
fn into_ipnet() {
    let net: Ipv4Net = cidr!("10.0.0.0/8").into();
    assert_eq!(net, "10.0.0.0/8".parse::<Ipv4Net>().unwrap());
    let net: Ipv6Net = cidr!("2001:db8::/32").into();
    assert_eq!(net, "2001:db8::/32".parse::<Ipv6Net>().unwrap());
}
