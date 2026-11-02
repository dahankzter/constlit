//! [`Capability`] and [`CapSet`], the values of `linux::cap!` and `linux::caps!`.

use core::fmt;

/// Capability names by number (stable kernel ABI, `CAP_CHOWN` = 0 through
/// `CAP_CHECKPOINT_RESTORE` = 40).
const NAMES: [&str; 41] = [
    "CAP_CHOWN",
    "CAP_DAC_OVERRIDE",
    "CAP_DAC_READ_SEARCH",
    "CAP_FOWNER",
    "CAP_FSETID",
    "CAP_KILL",
    "CAP_SETGID",
    "CAP_SETUID",
    "CAP_SETPCAP",
    "CAP_LINUX_IMMUTABLE",
    "CAP_NET_BIND_SERVICE",
    "CAP_NET_BROADCAST",
    "CAP_NET_ADMIN",
    "CAP_NET_RAW",
    "CAP_IPC_LOCK",
    "CAP_IPC_OWNER",
    "CAP_SYS_MODULE",
    "CAP_SYS_RAWIO",
    "CAP_SYS_CHROOT",
    "CAP_SYS_PTRACE",
    "CAP_SYS_PACCT",
    "CAP_SYS_ADMIN",
    "CAP_SYS_BOOT",
    "CAP_SYS_NICE",
    "CAP_SYS_RESOURCE",
    "CAP_SYS_TIME",
    "CAP_SYS_TTY_CONFIG",
    "CAP_MKNOD",
    "CAP_LEASE",
    "CAP_AUDIT_WRITE",
    "CAP_AUDIT_CONTROL",
    "CAP_SETFCAP",
    "CAP_MAC_OVERRIDE",
    "CAP_MAC_ADMIN",
    "CAP_SYSLOG",
    "CAP_WAKE_ALARM",
    "CAP_BLOCK_SUSPEND",
    "CAP_AUDIT_READ",
    "CAP_PERFMON",
    "CAP_BPF",
    "CAP_CHECKPOINT_RESTORE",
];

/// Bits of every known capability.
const ALL_BITS: u64 = (1 << NAMES.len()) - 1;

/// A Linux capability, usually written with [`cap!`](crate::linux::cap).
///
/// `Display` writes the name (`CAP_NET_ADMIN`); `Ord` goes by number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Capability(u8);

impl Capability {
    /// The capability with this number, or `None` past the last one constlit knows (40).
    pub const fn from_number(number: u8) -> Option<Self> {
        if (number as usize) < NAMES.len() {
            Some(Self(number))
        } else {
            None
        }
    }

    #[doc(hidden)]
    pub const fn __new_unchecked(number: u8) -> Self {
        Self(number)
    }

    /// The capability's number, as `capset(2)` and the bit masks use it.
    pub const fn number(self) -> u8 {
        self.0
    }

    /// The capability's name, like `"CAP_NET_ADMIN"`.
    pub const fn name(self) -> &'static str {
        NAMES[self.0 as usize]
    }
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A set of Linux capabilities as a bit mask by number, usually written with
/// [`caps!`](crate::linux::caps).
///
/// `Display` writes the names in number order, separated by `", "`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CapSet(u64);

impl CapSet {
    /// The empty set.
    pub const EMPTY: Self = Self(0);

    /// The set with these bits, or `None` if a bit past the last known capability is set.
    pub const fn from_bits(bits: u64) -> Option<Self> {
        if bits & !ALL_BITS == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    #[doc(hidden)]
    pub const fn __from_bits_unchecked(bits: u64) -> Self {
        Self(bits)
    }

    /// The bit mask: bit `n` is set when capability `n` is in the set.
    pub const fn bits(self) -> u64 {
        self.0
    }

    /// Whether `cap` is in the set.
    pub const fn contains(self, cap: Capability) -> bool {
        self.0 & (1 << cap.0) != 0
    }

    /// The set with `cap` added.
    #[must_use]
    pub const fn with(self, cap: Capability) -> Self {
        Self(self.0 | (1 << cap.0))
    }

    /// The set with `cap` removed.
    #[must_use]
    pub const fn without(self, cap: Capability) -> Self {
        Self(self.0 & !(1 << cap.0))
    }

    /// Capabilities in either set.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Capabilities in both sets.
    #[must_use]
    pub const fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    /// Capabilities in `self` but not in `other`.
    #[must_use]
    pub const fn difference(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }

    /// The number of capabilities in the set.
    pub const fn len(self) -> u32 {
        self.0.count_ones()
    }

    /// Whether the set is empty.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// The capabilities in the set, in number order.
    pub const fn iter(self) -> CapSetIter {
        CapSetIter(self.0)
    }
}

/// The capabilities in a [`CapSet`], in number order.
#[derive(Clone, Debug)]
pub struct CapSetIter(u64);

impl Iterator for CapSetIter {
    type Item = Capability;

    fn next(&mut self) -> Option<Capability> {
        if self.0 == 0 {
            return None;
        }
        let number = self.0.trailing_zeros() as u8;
        self.0 &= self.0 - 1;
        Some(Capability(number))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.0.count_ones() as usize;
        (n, Some(n))
    }
}

impl ExactSizeIterator for CapSetIter {}

impl IntoIterator for CapSet {
    type Item = Capability;
    type IntoIter = CapSetIter;

    fn into_iter(self) -> CapSetIter {
        self.iter()
    }
}

impl FromIterator<Capability> for CapSet {
    fn from_iter<I: IntoIterator<Item = Capability>>(caps: I) -> Self {
        caps.into_iter().fold(Self::EMPTY, Self::with)
    }
}

impl fmt::Display for CapSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, cap) in self.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            f.write_str(cap.name())?;
        }
        Ok(())
    }
}
