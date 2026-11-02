//! [`DevNum`], the value of `linux::dev!`.

use core::fmt;

/// A Linux device number, usually written with [`dev!`](crate::linux::dev): a
/// major number (0-4095) and a minor number (0-1048575), the kernel's limits.
///
/// `Display` writes `major:minor`, the form device cgroup rules, `lsblk` and
/// `/sys/dev` use.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DevNum {
    major: u32,
    minor: u32,
}

impl DevNum {
    /// The device number, or `None` if either part is beyond the kernel's limits.
    pub const fn new(major: u32, minor: u32) -> Option<Self> {
        if major <= 4095 && minor <= 1_048_575 {
            Some(Self { major, minor })
        } else {
            None
        }
    }

    #[doc(hidden)]
    pub const fn __new_unchecked(major: u32, minor: u32) -> Self {
        Self { major, minor }
    }

    /// The major number: the driver.
    pub const fn major(&self) -> u32 {
        self.major
    }

    /// The minor number: the device within the driver.
    pub const fn minor(&self) -> u32 {
        self.minor
    }

    /// The `dev_t` value, in glibc's `makedev` encoding (as `stat` reports it).
    pub const fn to_dev_t(&self) -> u64 {
        let (major, minor) = (self.major as u64, self.minor as u64);
        ((major & 0xffff_f000) << 32)
            | ((major & 0xfff) << 8)
            | ((minor & 0xffff_ff00) << 12)
            | (minor & 0xff)
    }
}

impl fmt::Display for DevNum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.major, self.minor)
    }
}
