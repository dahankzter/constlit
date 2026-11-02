//! [`Platform`], the value of `oci::platform!`.

use core::fmt;

/// An OCI platform, usually written with [`platform!`](crate::oci::platform):
/// an operating system and architecture with Go's names, and an optional variant.
///
/// Platforms compare as written: `linux/arm64` and `linux/arm64/v8` differ,
/// although container runtimes treat `v8` as arm64's default.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Platform {
    os: &'static str,
    arch: &'static str,
    variant: Option<&'static str>,
}

impl Platform {
    #[doc(hidden)]
    pub const fn __new_unchecked(
        os: &'static str,
        arch: &'static str,
        variant: Option<&'static str>,
    ) -> Self {
        Self { os, arch, variant }
    }

    /// The operating system, a Go `GOOS` name like `"linux"`.
    pub const fn os(&self) -> &'static str {
        self.os
    }

    /// The architecture, a Go `GOARCH` name like `"arm64"`.
    pub const fn arch(&self) -> &'static str {
        self.arch
    }

    /// The variant, like `"v8"`, if the literal has one.
    pub const fn variant(&self) -> Option<&'static str> {
        self.variant
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.os, self.arch)?;
        if let Some(variant) = self.variant {
            write!(f, "/{variant}")?;
        }
        Ok(())
    }
}
