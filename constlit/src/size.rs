//! [`ByteSize`], the value of `size!`.

use core::fmt;

/// A number of bytes, usually written with [`size!`](crate::size).
///
/// `Display` picks the largest 1024-based unit (`KiB`, `MiB`, ...) the value
/// reaches, with up to two decimals; a value that doesn't fit two decimals
/// exactly is rounded and shown with a leading `~`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ByteSize(u64);

impl ByteSize {
    /// A size of `bytes` bytes.
    pub const fn from_bytes(bytes: u64) -> Self {
        Self(bytes)
    }

    /// The size in bytes.
    pub const fn bytes(self) -> u64 {
        self.0
    }
}

impl From<ByteSize> for u64 {
    fn from(size: ByteSize) -> u64 {
        size.0
    }
}

impl fmt::Display for ByteSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const UNITS: [&str; 6] = ["KiB", "MiB", "GiB", "TiB", "PiB", "EiB"];
        if self.0 < 1024 {
            return write!(f, "{} B", self.0);
        }
        let power = (63 - self.0.leading_zeros()) / 10; // 1..=6
        let unit = 1u128 << (10 * power);
        let scaled = u128::from(self.0) * 100;
        let exact = scaled % unit == 0;
        let hundredths = (scaled + unit / 2) / unit;
        let (int, frac) = (hundredths / 100, hundredths % 100);
        let name = UNITS[power as usize - 1];
        let approx = if exact { "" } else { "~" };
        match frac {
            0 => write!(f, "{approx}{int} {name}"),
            _ if frac % 10 == 0 => write!(f, "{approx}{int}.{} {name}", frac / 10),
            _ => write!(f, "{approx}{int}.{frac:02} {name}"),
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use std::string::ToString;

    #[test]
    fn display() {
        assert_eq!(ByteSize::from_bytes(65536).to_string(), "64 KiB");
        assert_eq!(ByteSize::from_bytes(1536).to_string(), "1.5 KiB");
        assert_eq!(ByteSize::from_bytes(1000).to_string(), "1000 B");
        assert_eq!(ByteSize::from_bytes(0).to_string(), "0 B");
        assert_eq!(ByteSize::from_bytes(1100).to_string(), "~1.07 KiB");
        assert_eq!(ByteSize::from_bytes(1280).to_string(), "1.25 KiB");
        assert_eq!(ByteSize::from_bytes(3 << 30).to_string(), "3 GiB");
        assert_eq!(ByteSize::from_bytes(u64::MAX).to_string(), "~16 EiB");
    }

    #[test]
    fn accessors() {
        let size = ByteSize::from_bytes(4096);
        assert_eq!(size.bytes(), 4096);
        assert_eq!(u64::from(size), 4096);
        assert!(ByteSize::from_bytes(1) < size);
    }
}
