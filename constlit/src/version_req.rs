//! [`VersionReq`], the value of `version_req!`.

use core::cmp::Ordering;
use core::fmt;
use core::hash::{Hash, Hasher};

use crate::Version;
use crate::version::cmp_pre;

/// A comparator's operator, as in Cargo and the `semver` crate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Op {
    /// `=1.2.3`
    Exact,
    /// `>1.2.3`
    Greater,
    /// `>=1.2.3`
    GreaterEq,
    /// `<1.2.3`
    Less,
    /// `<=1.2.3`
    LessEq,
    /// `~1.2.3`
    Tilde,
    /// `^1.2.3`, and a bare `1.2.3`
    Caret,
    /// `1.*`, `1.2.x`
    Wildcard,
}

/// One comparator of a [`VersionReq`]: an operator and the version parts given.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Comparator {
    op: Op,
    major: u64,
    minor: Option<u64>,
    patch: Option<u64>,
    pre: &'static str,
}

impl Comparator {
    #[doc(hidden)]
    pub const fn __new_unchecked(
        op: Op,
        major: u64,
        minor: Option<u64>,
        patch: Option<u64>,
        pre: &'static str,
    ) -> Self {
        Self {
            op,
            major,
            minor,
            patch,
            pre,
        }
    }

    /// The operator.
    pub const fn op(&self) -> Op {
        self.op
    }

    /// The major version.
    pub const fn major(&self) -> u64 {
        self.major
    }

    /// The minor version, if given.
    pub const fn minor(&self) -> Option<u64> {
        self.minor
    }

    /// The patch version, if given.
    pub const fn patch(&self) -> Option<u64> {
        self.patch
    }

    /// The pre-release, or `""`.
    pub const fn pre(&self) -> &'static str {
        self.pre
    }

    /// Whether `v` satisfies this comparator alone (semver 1.x's rules). Unlike
    /// [`VersionReq::matches`], this applies no pre-release opt-in.
    pub fn matches(&self, v: &Version) -> bool {
        match self.op {
            Op::Exact | Op::Wildcard => self.matches_exact(v),
            Op::Greater => self.matches_ordered(v, Ordering::Greater),
            Op::GreaterEq => self.matches_exact(v) || self.matches_ordered(v, Ordering::Greater),
            Op::Less => self.matches_ordered(v, Ordering::Less),
            Op::LessEq => self.matches_exact(v) || self.matches_ordered(v, Ordering::Less),
            Op::Tilde => self.matches_tilde(v),
            Op::Caret => self.matches_caret(v),
        }
    }

    fn matches_exact(&self, v: &Version) -> bool {
        v.major() == self.major
            && self.minor.is_none_or(|minor| v.minor() == minor)
            && self.patch.is_none_or(|patch| v.patch() == patch)
            && v.pre() == self.pre
    }

    /// `>` with `want == Greater`, `<` with `want == Less`.
    fn matches_ordered(&self, v: &Version, want: Ordering) -> bool {
        if v.major() != self.major {
            return v.major().cmp(&self.major) == want;
        }
        let Some(minor) = self.minor else {
            return false;
        };
        if v.minor() != minor {
            return v.minor().cmp(&minor) == want;
        }
        let Some(patch) = self.patch else {
            return false;
        };
        if v.patch() != patch {
            return v.patch().cmp(&patch) == want;
        }
        cmp_pre(v.pre(), self.pre) == want
    }

    fn matches_tilde(&self, v: &Version) -> bool {
        if v.major() != self.major {
            return false;
        }
        if let Some(minor) = self.minor
            && v.minor() != minor
        {
            return false;
        }
        if let Some(patch) = self.patch
            && v.patch() != patch
        {
            return v.patch() > patch;
        }
        cmp_pre(v.pre(), self.pre) != Ordering::Less
    }

    fn matches_caret(&self, v: &Version) -> bool {
        if v.major() != self.major {
            return false;
        }
        let Some(minor) = self.minor else {
            return true;
        };
        let Some(patch) = self.patch else {
            return if self.major > 0 {
                v.minor() >= minor
            } else {
                v.minor() == minor
            };
        };
        if self.major > 0 {
            if v.minor() != minor {
                return v.minor() > minor;
            } else if v.patch() != patch {
                return v.patch() > patch;
            }
        } else if minor > 0 {
            if v.minor() != minor {
                return false;
            } else if v.patch() != patch {
                return v.patch() > patch;
            }
        } else if v.minor() != minor || v.patch() != patch {
            return false;
        }
        cmp_pre(v.pre(), self.pre) != Ordering::Less
    }

    /// semver's opt-in for pre-releases: the same `MAJOR.MINOR.PATCH` with a pre-release.
    fn allows_pre_of(&self, v: &Version) -> bool {
        self.major == v.major()
            && self.minor == Some(v.minor())
            && self.patch == Some(v.patch())
            && !self.pre.is_empty()
    }
}

/// A Cargo version requirement, usually written with [`version_req!`](crate::version_req).
///
/// [`matches`](Self::matches) follows the `semver` crate (and so Cargo), including
/// its pre-release rule: a version with a pre-release matches only if some
/// comparator names the same `MAJOR.MINOR.PATCH` with a pre-release.
/// `Display` writes the literal; `Eq` and `Hash` compare the comparators, so
/// `">=1.2"` equals `">= 1.2"`.
#[derive(Clone, Copy, Debug)]
pub struct VersionReq {
    comparators: &'static [Comparator],
    text: &'static str,
}

impl VersionReq {
    #[doc(hidden)]
    pub const fn __new_unchecked(comparators: &'static [Comparator], text: &'static str) -> Self {
        Self { comparators, text }
    }

    /// The comparators; empty for `"*"`, which matches every release.
    pub const fn comparators(&self) -> &'static [Comparator] {
        self.comparators
    }

    /// The literal as written.
    pub const fn as_str(&self) -> &'static str {
        self.text
    }

    /// Whether `version` satisfies every comparator, as Cargo decides it.
    pub fn matches(&self, version: &Version) -> bool {
        self.comparators.iter().all(|c| c.matches(version))
            && (version.pre().is_empty()
                || self.comparators.iter().any(|c| c.allows_pre_of(version)))
    }

    /// The requirement as a [`semver::VersionReq`].
    #[cfg(feature = "semver")]
    #[cfg_attr(docsrs, doc(cfg(feature = "semver")))]
    pub fn to_semver(&self) -> semver::VersionReq {
        let comparators = self
            .comparators
            .iter()
            .map(|c| semver::Comparator {
                op: match c.op {
                    Op::Exact => semver::Op::Exact,
                    Op::Greater => semver::Op::Greater,
                    Op::GreaterEq => semver::Op::GreaterEq,
                    Op::Less => semver::Op::Less,
                    Op::LessEq => semver::Op::LessEq,
                    Op::Tilde => semver::Op::Tilde,
                    Op::Caret => semver::Op::Caret,
                    Op::Wildcard => semver::Op::Wildcard,
                },
                major: c.major,
                minor: c.minor,
                patch: c.patch,
                pre: semver::Prerelease::new(c.pre).expect("checked by version_req!"),
            })
            .collect();
        semver::VersionReq { comparators }
    }
}

impl PartialEq for VersionReq {
    fn eq(&self, other: &Self) -> bool {
        self.comparators == other.comparators
    }
}

impl Eq for VersionReq {}

impl Hash for VersionReq {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.comparators.hash(state);
    }
}

impl fmt::Display for VersionReq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.text)
    }
}
