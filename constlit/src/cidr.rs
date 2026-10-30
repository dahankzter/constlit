//! [`Ipv4Cidr`], [`Ipv6Cidr`] and [`Cidr`], the values of `cidr!`.

use core::fmt;
use core::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Why a network address and prefix length don't form a valid network.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CidrError {
    /// The prefix is longer than the address (32 bits for IPv4, 128 for IPv6).
    PrefixTooLong { max: u8 },
    /// The address has bits set after the prefix, so it isn't a network address.
    HostBitsSet,
}

impl fmt::Display for CidrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CidrError::PrefixTooLong { max } => write!(f, "prefix is too long (max {max})"),
            CidrError::HostBitsSet => f.write_str("host bits are set"),
        }
    }
}

impl core::error::Error for CidrError {}

macro_rules! cidr_type {
    ($(#[$doc:meta])* $name:ident, $addr:ty, $bits:ty) => {
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name {
            addr: $addr,
            prefix: u8,
        }

        impl $name {
            const MAX: u8 = <$bits>::BITS as u8;

            /// The network `addr/prefix`; `addr` must have no bits set after the prefix.
            pub const fn new(addr: $addr, prefix: u8) -> Result<Self, CidrError> {
                if prefix > Self::MAX {
                    return Err(CidrError::PrefixTooLong { max: Self::MAX });
                }
                if addr.to_bits() & !Self::mask_bits(prefix) != 0 {
                    return Err(CidrError::HostBitsSet);
                }
                Ok(Self { addr, prefix })
            }

            /// Used by `cidr!`, which has already checked the address and prefix.
            #[doc(hidden)]
            pub const fn __new_unchecked(addr: $addr, prefix: u8) -> Self {
                Self { addr, prefix }
            }

            const fn mask_bits(prefix: u8) -> $bits {
                match prefix {
                    0 => 0,
                    _ => <$bits>::MAX << (Self::MAX - prefix),
                }
            }

            /// The network address, the first address in the network.
            pub const fn network(&self) -> $addr {
                self.addr
            }

            /// The prefix length in bits.
            pub const fn prefix(&self) -> u8 {
                self.prefix
            }

            /// The netmask: `prefix` one-bits followed by zero-bits.
            pub const fn netmask(&self) -> $addr {
                <$addr>::from_bits(Self::mask_bits(self.prefix))
            }

            /// Whether `ip` is in this network.
            pub const fn contains(&self, ip: $addr) -> bool {
                ip.to_bits() & Self::mask_bits(self.prefix) == self.addr.to_bits()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}/{}", self.addr, self.prefix)
            }
        }
    };
}

cidr_type!(
    /// An IPv4 network such as `10.0.0.0/8`, usually written with [`cidr!`](crate::cidr).
    Ipv4Cidr,
    Ipv4Addr,
    u32
);

cidr_type!(
    /// An IPv6 network such as `2001:db8::/32`, usually written with [`cidr!`](crate::cidr).
    Ipv6Cidr,
    Ipv6Addr,
    u128
);

impl Ipv4Cidr {
    /// The broadcast address, the last address in the network.
    pub const fn broadcast(&self) -> Ipv4Addr {
        Ipv4Addr::from_bits(self.addr.to_bits() | !Self::mask_bits(self.prefix))
    }
}

/// An IPv4 or IPv6 network.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Cidr {
    V4(Ipv4Cidr),
    V6(Ipv6Cidr),
}

impl Cidr {
    /// Whether `ip` is in this network; an address of the other family never is.
    pub const fn contains(&self, ip: IpAddr) -> bool {
        match (self, ip) {
            (Cidr::V4(net), IpAddr::V4(ip)) => net.contains(ip),
            (Cidr::V6(net), IpAddr::V6(ip)) => net.contains(ip),
            _ => false,
        }
    }
}

impl From<Ipv4Cidr> for Cidr {
    fn from(net: Ipv4Cidr) -> Self {
        Cidr::V4(net)
    }
}

impl From<Ipv6Cidr> for Cidr {
    fn from(net: Ipv6Cidr) -> Self {
        Cidr::V6(net)
    }
}

impl fmt::Display for Cidr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Cidr::V4(net) => net.fmt(f),
            Cidr::V6(net) => net.fmt(f),
        }
    }
}

#[cfg(feature = "ipnet")]
impl From<Ipv4Cidr> for ipnet::Ipv4Net {
    fn from(net: Ipv4Cidr) -> Self {
        ipnet::Ipv4Net::new(net.addr, net.prefix).expect("prefix checked on construction")
    }
}

#[cfg(feature = "ipnet")]
impl From<Ipv6Cidr> for ipnet::Ipv6Net {
    fn from(net: Ipv6Cidr) -> Self {
        ipnet::Ipv6Net::new(net.addr, net.prefix).expect("prefix checked on construction")
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use core::net::{IpAddr, Ipv4Addr, Ipv6Addr};
    use std::string::ToString;

    const fn v4(a: u8, b: u8, c: u8, d: u8) -> Ipv4Addr {
        Ipv4Addr::new(a, b, c, d)
    }

    #[test]
    fn cidr_corners() {
        let all = Ipv4Cidr::new(v4(0, 0, 0, 0), 0).unwrap();
        assert!(all.contains(v4(255, 255, 255, 255)));
        assert!(all.contains(v4(0, 0, 0, 0)));
        assert_eq!(all.netmask(), v4(0, 0, 0, 0));

        let one = Ipv4Cidr::new(v4(10, 0, 0, 1), 32).unwrap();
        assert!(one.contains(v4(10, 0, 0, 1)));
        assert!(!one.contains(v4(10, 0, 0, 2)));
        assert_eq!(one.broadcast(), v4(10, 0, 0, 1));

        assert!(
            Ipv6Cidr::new(Ipv6Addr::UNSPECIFIED, 0)
                .unwrap()
                .contains(Ipv6Addr::LOCALHOST)
        );
        let lo = Ipv6Cidr::new(Ipv6Addr::LOCALHOST, 128).unwrap();
        assert!(lo.contains(Ipv6Addr::LOCALHOST));
        assert!(!lo.contains(Ipv6Addr::UNSPECIFIED));

        assert_eq!(
            Ipv4Cidr::new(v4(10, 1, 0, 0), 8),
            Err(CidrError::HostBitsSet)
        );
        assert_eq!(
            Ipv4Cidr::new(v4(10, 0, 0, 0), 33),
            Err(CidrError::PrefixTooLong { max: 32 })
        );
        assert_eq!(
            Ipv6Cidr::new(Ipv6Addr::UNSPECIFIED, 129),
            Err(CidrError::PrefixTooLong { max: 128 })
        );

        let ten = Ipv4Cidr::new(v4(10, 0, 0, 0), 8).unwrap();
        assert_eq!(ten.broadcast(), v4(10, 255, 255, 255));
        assert_eq!(ten.netmask(), v4(255, 0, 0, 0));
        assert_eq!(ten.network(), v4(10, 0, 0, 0));
        assert_eq!(ten.prefix(), 8);
        assert_eq!(ten.to_string(), "10.0.0.0/8");
    }

    #[test]
    fn either_family() {
        let ten: Cidr = Ipv4Cidr::new(v4(10, 0, 0, 0), 8).unwrap().into();
        let doc: Cidr = Ipv6Cidr::new("2001:db8::".parse().unwrap(), 32)
            .unwrap()
            .into();
        assert!(ten.contains(IpAddr::V4(v4(10, 9, 9, 9))));
        assert!(!ten.contains(IpAddr::V6("::ffff:10.9.9.9".parse().unwrap())));
        assert!(doc.contains(IpAddr::V6("2001:db8::1".parse().unwrap())));
        assert!(!doc.contains(IpAddr::V4(v4(10, 0, 0, 1))));
        assert_eq!(doc.to_string(), "2001:db8::/32");
        assert_eq!(CidrError::HostBitsSet.to_string(), "host bits are set");
        assert_eq!(
            CidrError::PrefixTooLong { max: 32 }.to_string(),
            "prefix is too long (max 32)"
        );
    }
}
