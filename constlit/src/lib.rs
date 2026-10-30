//! Compile-time checked literals that become typed constants.
#![no_std]

mod cidr;
mod size;

pub use cidr::{Cidr, CidrError, Ipv4Cidr, Ipv6Cidr};
pub use size::ByteSize;

/// A duration literal, checked at compile time, as a [`core::time::Duration`].
///
/// Units: `ns`, `us` (or `µs`), `ms`, `s`, `m`, `h`, `d` (exactly 24 hours).
/// Components go from largest to smallest, optionally separated by single spaces.
/// Decimals are allowed when the result is a whole number of nanoseconds.
///
/// ```
/// use core::time::Duration;
/// const TIMEOUT: Duration = constlit::dur!("1h30m");
/// assert_eq!(TIMEOUT, Duration::from_secs(5400));
/// assert_eq!(constlit::dur!("1.5s"), Duration::from_millis(1500));
/// ```
///
/// A bare number, an unknown unit, or units out of order is a compile error:
///
/// ```compile_fail
/// let _ = constlit::dur!("30");
/// ```
pub use constlit_macros::dur;

/// A byte-size literal, checked at compile time, as a [`ByteSize`].
///
/// Units are strict and case-sensitive:
/// - `KiB`, `MiB`, `GiB`, `TiB`, `PiB` (and Kubernetes' `Ki` ... `Pi`) are powers of 1024;
/// - `KB`, `MB`, `GB`, `TB`, `PB` are powers of 1000;
/// - `B`, or no unit, is bytes.
///
/// ```
/// use constlit::{ByteSize, size};
/// const LIMIT: ByteSize = size!("64 KiB");
/// assert_eq!(LIMIT.bytes(), 65536);
/// assert_eq!(size!("1.5 KB").bytes(), 1500);
/// ```
///
/// Any other spelling, like `kb`, is a compile error asking which one you meant:
///
/// ```compile_fail
/// let _ = constlit::size!("64 kb");
/// ```
pub use constlit_macros::size;

/// An IP address literal, checked at compile time.
///
/// Dotted IPv4 becomes a [`core::net::Ipv4Addr`]; any standard IPv6 form becomes
/// a [`core::net::Ipv6Addr`]. Use `.into()` for an `IpAddr`.
///
/// ```
/// use core::net::{Ipv4Addr, Ipv6Addr};
/// const DNS: Ipv4Addr = constlit::ip!("10.0.0.53");
/// const LOOPBACK: Ipv6Addr = constlit::ip!("::1");
/// assert_eq!(LOOPBACK, Ipv6Addr::LOCALHOST);
/// ```
///
/// Leading zeros (`"010.0.0.1"`: decimal or octal?) and zone suffixes are compile errors:
///
/// ```compile_fail
/// let _ = constlit::ip!("010.0.0.1");
/// ```
pub use constlit_macros::ip;

/// A socket address literal, checked at compile time.
///
/// `"ipv4:port"` becomes a [`core::net::SocketAddrV4`]; `"[ipv6]:port"` becomes a
/// [`core::net::SocketAddrV6`]. Hostnames are compile errors: they need a DNS lookup.
///
/// ```
/// use core::net::SocketAddrV4;
/// const LISTEN: SocketAddrV4 = constlit::addr!("0.0.0.0:8080");
/// assert_eq!(LISTEN.port(), 8080);
/// ```
///
/// ```compile_fail
/// let _ = constlit::addr!("localhost:80");
/// ```
pub use constlit_macros::addr;

/// A network literal, checked at compile time, as an [`Ipv4Cidr`] or [`Ipv6Cidr`].
///
/// The address must be the network address: host bits set after the prefix are
/// a compile error that suggests the network you probably meant.
///
/// ```
/// use constlit::{Ipv4Cidr, cidr};
/// const PRIVATE: Ipv4Cidr = cidr!("10.0.0.0/8");
/// assert!(PRIVATE.contains(constlit::ip!("10.1.2.3")));
/// assert_eq!(PRIVATE.broadcast(), constlit::ip!("10.255.255.255"));
/// ```
///
/// ```compile_fail
/// let _ = constlit::cidr!("10.1.2.3/8"); // did you mean "10.0.0.0/8"?
/// ```
pub use constlit_macros::cidr;
