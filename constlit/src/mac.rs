//! [`MacAddr`], the value of `mac!`.

use core::fmt;

/// A MAC address (EUI-48), usually written with [`mac!`](crate::mac).
///
/// Every 6-byte array is a valid address, so [`MacAddr::new`] is unchecked.
/// `Display` writes lowercase hex bytes separated by colons.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MacAddr([u8; 6]);

impl MacAddr {
    /// The address with these six bytes.
    pub const fn new(octets: [u8; 6]) -> Self {
        Self(octets)
    }

    /// The six bytes.
    pub const fn octets(self) -> [u8; 6] {
        self.0
    }

    /// True for group addresses (the lowest bit of the first byte is set),
    /// including the broadcast address `ff:ff:ff:ff:ff:ff`.
    pub const fn is_multicast(self) -> bool {
        self.0[0] & 1 != 0
    }

    /// True for individual addresses: the opposite of [`is_multicast`](Self::is_multicast).
    pub const fn is_unicast(self) -> bool {
        !self.is_multicast()
    }

    /// True for locally administered addresses (the second-lowest bit of the first byte is set).
    pub const fn is_local(self) -> bool {
        self.0[0] & 2 != 0
    }

    /// True for universally administered (vendor-assigned) addresses: the opposite of
    /// [`is_local`](Self::is_local).
    pub const fn is_universal(self) -> bool {
        !self.is_local()
    }
}

impl From<[u8; 6]> for MacAddr {
    fn from(octets: [u8; 6]) -> Self {
        Self(octets)
    }
}

impl From<MacAddr> for [u8; 6] {
    fn from(mac: MacAddr) -> Self {
        mac.0
    }
}

impl fmt::Display for MacAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [a, b, c, d, e, g] = self.0;
        write!(f, "{a:02x}:{b:02x}:{c:02x}:{d:02x}:{e:02x}:{g:02x}")
    }
}
