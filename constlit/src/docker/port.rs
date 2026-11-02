//! [`PortMapping`], the value of `docker::port!`.

use core::fmt;
use core::net::IpAddr;

/// A transport protocol a port mapping applies to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Protocol {
    Tcp,
    Udp,
    Sctp,
}

impl Protocol {
    /// `"tcp"`, `"udp"` or `"sctp"`.
    pub const fn name(self) -> &'static str {
        match self {
            Protocol::Tcp => "tcp",
            Protocol::Udp => "udp",
            Protocol::Sctp => "sctp",
        }
    }
}

/// An inclusive range of ports; a single port is a range of one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PortRange {
    start: u16,
    end: u16,
}

impl PortRange {
    #[doc(hidden)]
    pub const fn __new_unchecked(start: u16, end: u16) -> Self {
        Self { start, end }
    }

    /// The first port.
    pub const fn start(&self) -> u16 {
        self.start
    }

    /// The last port (equal to [`start`](Self::start) for a single port).
    pub const fn end(&self) -> u16 {
        self.end
    }

    /// The number of ports, at least 1.
    pub const fn count(&self) -> u32 {
        (self.end - self.start) as u32 + 1
    }
}

impl fmt::Display for PortRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.start == self.end {
            write!(f, "{}", self.start)
        } else {
            write!(f, "{}-{}", self.start, self.end)
        }
    }
}

/// A Docker port mapping, usually written with [`port!`](crate::docker::port): which
/// container ports to publish, on which host ports and address.
///
/// `Display` writes a canonical form: the host IP (IPv6 in brackets), the host
/// ports, the container ports and always the protocol, like `"127.0.0.1:8080:80/tcp"`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PortMapping {
    host_ip: Option<IpAddr>,
    host: Option<PortRange>,
    container: PortRange,
    protocol: Protocol,
}

impl PortMapping {
    #[doc(hidden)]
    pub const fn __new_unchecked(
        host_ip: Option<IpAddr>,
        host: Option<PortRange>,
        container: PortRange,
        protocol: Protocol,
    ) -> Self {
        Self {
            host_ip,
            host,
            container,
            protocol,
        }
    }

    /// The host address to bind, if the mapping names one.
    pub const fn host_ip(&self) -> Option<IpAddr> {
        self.host_ip
    }

    /// The host ports, if the mapping names them; otherwise Docker picks a free port.
    pub const fn host(&self) -> Option<PortRange> {
        self.host
    }

    /// The container ports.
    pub const fn container(&self) -> PortRange {
        self.container
    }

    /// The protocol, `tcp` unless the mapping says otherwise.
    pub const fn protocol(&self) -> Protocol {
        self.protocol
    }
}

impl fmt::Display for PortMapping {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.host_ip {
            Some(IpAddr::V6(ip)) => write!(f, "[{ip}]:")?,
            Some(IpAddr::V4(ip)) => write!(f, "{ip}:")?,
            None => {}
        }
        match self.host {
            Some(host) => write!(f, "{host}:")?,
            None if self.host_ip.is_some() => f.write_str(":")?,
            None => {}
        }
        write!(f, "{}/{}", self.container, self.protocol.name())
    }
}
