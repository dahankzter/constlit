//! [`ImageRef`], the value of `oci::image!`.

use core::fmt;
use core::hash::{Hash, Hasher};

use super::Digest;

/// A container image reference, usually written with [`image!`](crate::oci::image).
///
/// The registry, repository and canonical form are worked out at compile time,
/// the way Docker normalizes names: `"nginx"` is
/// `docker.io/library/nginx:latest`. `Display` writes the literal as written;
/// `Eq` and `Hash` compare [`canonical`](Self::canonical), so `image!("nginx")`
/// equals `image!("docker.io/library/nginx:latest")`.
#[derive(Clone, Copy, Debug)]
pub struct ImageRef {
    text: &'static str,
    registry: &'static str,
    repository: &'static str,
    tag: Option<&'static str>,
    digest: Option<Digest>,
    canonical: &'static str,
}

impl ImageRef {
    #[doc(hidden)]
    pub const fn __new_unchecked(
        text: &'static str,
        registry: &'static str,
        repository: &'static str,
        tag: Option<&'static str>,
        digest: Option<Digest>,
        canonical: &'static str,
    ) -> Self {
        Self {
            text,
            registry,
            repository,
            tag,
            digest,
            canonical,
        }
    }

    /// The registry: `"docker.io"` when the literal names none (or writes the
    /// legacy `index.docker.io`), otherwise the host as written, with its port.
    pub const fn registry(&self) -> &'static str {
        self.registry
    }

    /// The repository path, with `library/` added for official Docker Hub images.
    pub const fn repository(&self) -> &'static str {
        self.repository
    }

    /// The tag, if the literal has one.
    pub const fn tag(&self) -> Option<&'static str> {
        self.tag
    }

    /// The digest, if the literal has one.
    pub const fn digest(&self) -> Option<Digest> {
        self.digest
    }

    /// The literal as written.
    pub const fn as_str(&self) -> &'static str {
        self.text
    }

    /// The fully qualified reference, like `"docker.io/library/nginx:latest"`.
    /// `latest` is added only when the literal has neither a tag nor a digest.
    pub const fn canonical(&self) -> &'static str {
        self.canonical
    }
}

impl PartialEq for ImageRef {
    fn eq(&self, other: &Self) -> bool {
        self.canonical == other.canonical
    }
}

impl Eq for ImageRef {}

impl Hash for ImageRef {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.canonical.hash(state);
    }
}

impl fmt::Display for ImageRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.text)
    }
}
