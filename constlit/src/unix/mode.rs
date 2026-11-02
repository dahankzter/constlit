//! [`Mode`], the value of `unix::mode!`.

use core::fmt;

/// File permission bits (`0o7777` at most: setuid, setgid, sticky and `rwx` for
/// user, group and other), usually written with [`mode!`](crate::unix::mode).
///
/// `Display` writes 4-digit octal (`0755`); the alternate form `{:#}` writes the
/// 9-character `ls` form (`rwxr-xr-x`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Mode(u32);

impl Mode {
    /// The mode with these bits, or `None` if any bit above `0o7777` is set.
    pub const fn new(bits: u32) -> Option<Self> {
        if bits <= 0o7777 {
            Some(Self(bits))
        } else {
            None
        }
    }

    #[doc(hidden)]
    pub const fn __new_unchecked(bits: u32) -> Self {
        Self(bits)
    }

    /// The permission bits.
    pub const fn bits(self) -> u32 {
        self.0
    }

    /// The setuid bit (`0o4000`).
    pub const fn is_setuid(self) -> bool {
        self.0 & 0o4000 != 0
    }

    /// The setgid bit (`0o2000`).
    pub const fn is_setgid(self) -> bool {
        self.0 & 0o2000 != 0
    }

    /// The sticky bit (`0o1000`).
    pub const fn is_sticky(self) -> bool {
        self.0 & 0o1000 != 0
    }

    /// The user's `rwx` bits, `0..=7` (read 4, write 2, execute 1).
    pub const fn user(self) -> u8 {
        ((self.0 >> 6) & 7) as u8
    }

    /// The group's `rwx` bits, `0..=7`.
    pub const fn group(self) -> u8 {
        ((self.0 >> 3) & 7) as u8
    }

    /// Everyone else's `rwx` bits, `0..=7`.
    pub const fn other(self) -> u8 {
        (self.0 & 7) as u8
    }

    /// The mode as [`std::fs::Permissions`].
    #[cfg(all(feature = "std", unix))]
    #[cfg_attr(docsrs, doc(cfg(all(feature = "std", unix))))]
    pub fn to_permissions(self) -> std::fs::Permissions {
        use std::os::unix::fs::PermissionsExt;
        std::fs::Permissions::from_mode(self.0)
    }
}

impl From<Mode> for u32 {
    fn from(mode: Mode) -> u32 {
        mode.0
    }
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !f.alternate() {
            return write!(f, "{:04o}", self.0);
        }
        let specials = [(0o4000, 's', 'S'), (0o2000, 's', 'S'), (0o1000, 't', 'T')];
        for (class, (special, set, unset)) in specials.into_iter().enumerate() {
            let perm = (self.0 >> (6 - 3 * class)) & 7;
            let read = if perm & 4 != 0 { 'r' } else { '-' };
            let write = if perm & 2 != 0 { 'w' } else { '-' };
            let execute = match (perm & 1 != 0, self.0 & special != 0) {
                (true, true) => set,
                (false, true) => unset,
                (true, false) => 'x',
                (false, false) => '-',
            };
            write!(f, "{read}{write}{execute}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_api() {
        assert_eq!(Mode::new(0o10000), None);
        assert_eq!(Mode::new(0o755).unwrap().bits(), 0o755);
        assert_eq!(Mode::new(0o7777).unwrap().bits(), 0o7777);
    }
}
