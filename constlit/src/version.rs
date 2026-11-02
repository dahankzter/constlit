use core::cmp::Ordering;
use core::fmt;
use core::hash::{Hash, Hasher};

/// A SemVer 2.0.0 version, usually made with [`version!`](crate::version).
///
/// Comparison follows SemVer precedence: build metadata is ignored, so
/// `1.0.0+a == 1.0.0+b`, and `Hash` agrees with `Eq`.
#[derive(Clone, Copy, Debug)]
pub struct Version {
    major: u64,
    minor: u64,
    patch: u64,
    pre: &'static str,
    build: &'static str,
}

impl Version {
    /// `MAJOR.MINOR.PATCH`, without pre-release or build metadata.
    pub const fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self::__new_unchecked(major, minor, patch, "", "")
    }

    #[doc(hidden)]
    pub const fn __new_unchecked(
        major: u64,
        minor: u64,
        patch: u64,
        pre: &'static str,
        build: &'static str,
    ) -> Self {
        Self {
            major,
            minor,
            patch,
            pre,
            build,
        }
    }

    pub const fn major(&self) -> u64 {
        self.major
    }

    pub const fn minor(&self) -> u64 {
        self.minor
    }

    pub const fn patch(&self) -> u64 {
        self.patch
    }

    /// The pre-release identifiers without the leading `-`, or `""`.
    pub const fn pre(&self) -> &'static str {
        self.pre
    }

    /// The build metadata without the leading `+`, or `""`.
    pub const fn build(&self) -> &'static str {
        self.build
    }

    /// Converts to a [`semver::Version`].
    #[cfg(feature = "semver")]
    pub fn to_semver(&self) -> semver::Version {
        semver::Version {
            major: self.major,
            minor: self.minor,
            patch: self.patch,
            pre: semver::Prerelease::new(self.pre).expect("validated by version!"),
            build: semver::BuildMetadata::new(self.build).expect("validated by version!"),
        }
    }
}

/// SemVer precedence of two pre-release strings; `""` (a release) ranks highest.
pub(crate) fn cmp_pre(a: &str, b: &str) -> Ordering {
    match (a.is_empty(), b.is_empty()) {
        (true, true) => return Ordering::Equal,
        (true, false) => return Ordering::Greater,
        (false, true) => return Ordering::Less,
        (false, false) => {}
    }
    let mut left = a.split('.');
    let mut right = b.split('.');
    loop {
        let (x, y) = match (left.next(), right.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) => (x, y),
        };
        let numeric = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
        let order = match (numeric(x), numeric(y)) {
            // No leading zeros, so the longer number is larger, at any size.
            (true, true) => x.len().cmp(&y.len()).then_with(|| x.cmp(y)),
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            (false, false) => x.cmp(y),
        };
        if order != Ordering::Equal {
            return order;
        }
    }
}

impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Version {}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.major, self.minor, self.patch)
            .cmp(&(other.major, other.minor, other.patch))
            .then_with(|| cmp_pre(self.pre, other.pre))
    }
}

impl Hash for Version {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.major, self.minor, self.patch, self.pre).hash(state);
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if !self.pre.is_empty() {
            write!(f, "-{}", self.pre)?;
        }
        if !self.build.is_empty() {
            write!(f, "+{}", self.build)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(major: u64, minor: u64, patch: u64, pre: &'static str, build: &'static str) -> Version {
        Version::__new_unchecked(major, minor, patch, pre, build)
    }

    fn hash(v: &Version) -> u64 {
        use core::hash::{BuildHasher, BuildHasherDefault};
        BuildHasherDefault::<Fnv>::default().hash_one(v)
    }

    #[derive(Default)]
    struct Fnv(u64);

    impl core::hash::Hasher for Fnv {
        fn finish(&self) -> u64 {
            self.0
        }
        fn write(&mut self, bytes: &[u8]) {
            for &b in bytes {
                self.0 = (self.0 ^ u64::from(b)).wrapping_mul(0x100_0000_01b3);
            }
        }
    }

    #[test]
    fn precedence() {
        let chain = [
            v(1, 0, 0, "alpha", ""),
            v(1, 0, 0, "alpha.1", ""),
            v(1, 0, 0, "alpha.beta", ""),
            v(1, 0, 0, "beta", ""),
            v(1, 0, 0, "beta.2", ""),
            v(1, 0, 0, "beta.11", ""),
            v(1, 0, 0, "rc.1", ""),
            v(1, 0, 0, "", ""),
            v(2, 0, 0, "", ""),
            v(2, 1, 0, "", ""),
            v(2, 1, 1, "", ""),
        ];
        for pair in chain.windows(2) {
            assert!(pair[0] < pair[1], "{} < {}", pair[0], pair[1]);
        }
        assert_eq!(v(1, 0, 0, "", "a"), v(1, 0, 0, "", "b"));
        assert_eq!(hash(&v(1, 0, 0, "", "a")), hash(&v(1, 0, 0, "", "b")));
    }

    #[test]
    fn precedence_big_numeric() {
        let small = v(1, 0, 0, "alpha.99", "");
        let big = v(1, 0, 0, "alpha.99999999999999999999999", "");
        let alpha = v(1, 0, 0, "alpha.a", "");
        assert!(small < big);
        assert!(big < alpha);
    }

    #[test]
    fn display() {
        struct Buf([u8; 64], usize);
        impl core::fmt::Write for Buf {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                self.0[self.1..self.1 + s.len()].copy_from_slice(s.as_bytes());
                self.1 += s.len();
                Ok(())
            }
        }
        let cases = [
            (v(1, 2, 3, "", ""), "1.2.3"),
            (v(1, 2, 3, "rc.1", ""), "1.2.3-rc.1"),
            (v(1, 2, 3, "rc.1", "b.7"), "1.2.3-rc.1+b.7"),
        ];
        for (version, text) in cases {
            let mut buf = Buf([0; 64], 0);
            core::fmt::write(&mut buf, format_args!("{version}")).unwrap();
            assert_eq!(&buf.0[..buf.1], text.as_bytes());
        }
    }
}
