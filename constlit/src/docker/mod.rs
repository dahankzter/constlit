//! Docker values: port mappings.
//!
//! ```
//! use constlit::docker::{self, PortMapping, Protocol};
//!
//! const WEB: PortMapping = docker::port!("127.0.0.1:8080:80");
//! assert_eq!(WEB.container().start(), 80);
//! assert_eq!(WEB.protocol(), Protocol::Tcp);
//! assert_eq!(WEB.to_string(), "127.0.0.1:8080:80/tcp");
//! ```

mod port;

pub use port::{PortMapping, PortRange, Protocol};

/// A port mapping, checked at compile time, as a [`PortMapping`].
///
/// The `docker run -p` form, `[host_ip:][host_port:]container_port[/protocol]`:
/// ports may be ranges (`"8000-8010:8000-8010"`, or a host range with one
/// container port), an IPv6 host address goes in brackets, the host port may be
/// left empty after an address (`"127.0.0.1::80"`), and the protocol is `tcp`,
/// `udp` or `sctp`, in lower case, defaulting to `tcp`.
///
/// Spellings Docker quietly accepts but that read as mistakes are compile errors
/// with a suggestion: port 0, leading zeros, `+`, an upper-case protocol, an
/// unbracketed IPv6 address, an empty host port or protocol.
///
/// ```
/// use constlit::docker::{self, PortMapping};
/// const DNS: PortMapping = docker::port!("[::1]:53:53/udp");
/// assert!(DNS.host_ip().unwrap().is_ipv6());
/// ```
///
/// ```compile_fail
/// let _ = constlit::docker::port!("8080:80/TCP"); // write "8080:80/tcp"
/// ```
pub use constlit_macros::docker_port as port;
