//! Container values from the OCI image and distribution specs: content digests,
//! image references and platforms.
//!
//! ```
//! use constlit::oci::{self, Digest, DigestAlgorithm};
//!
//! const LAYER: Digest =
//!     oci::digest!("sha256:0000000000000000000000000000000000000000000000000000000000000000");
//! assert_eq!(LAYER.algorithm(), DigestAlgorithm::Sha256);
//! assert_eq!(LAYER.as_bytes(), &[0; 32]);
//! ```

mod digest;
mod image;
mod platform;

pub use digest::{Digest, DigestAlgorithm};
pub use image::ImageRef;
pub use platform::Platform;

/// A content digest, checked at compile time, as a [`Digest`].
///
/// `"sha256:"` followed by 64 lower-case hex digits, or `"sha512:"` followed by
/// 128, the two algorithms the OCI image spec registers. The bytes are decoded
/// while your crate compiles.
///
/// ```
/// use constlit::oci::{self, Digest};
/// const D: Digest =
///     oci::digest!("sha256:abababababababababababababababababababababababababababababababab");
/// assert_eq!(D.as_bytes()[0], 0xab);
/// ```
///
/// Upper-case hex, other algorithms and a missing `sha256:` are compile errors:
///
/// ```compile_fail
/// let _ = constlit::oci::digest!("abababababababababababababababababababababababababababababababab");
/// ```
pub use constlit_macros::oci_digest as digest;

/// An image reference, checked at compile time, as an [`ImageRef`].
///
/// `[registry/]repository[:tag][@digest]`, with the grammar and normalization of
/// Docker's `distribution/reference`:
/// - the first part is a registry when it is `localhost` or contains `.` or `:`;
///   otherwise the registry is `docker.io`;
/// - a Docker Hub name with no `/` gets `library/`, and `index.docker.io`
///   becomes `docker.io`;
/// - repository names are lower case; tags are up to 128 letters, digits, `_`,
///   `.` and `-`; a digest follows [`digest!`]'s rules.
///
/// ```
/// use constlit::oci::{self, ImageRef};
/// const WEB: ImageRef = oci::image!("nginx:1.27");
/// assert_eq!(WEB.canonical(), "docker.io/library/nginx:1.27");
/// assert_eq!(WEB.repository(), "library/nginx");
/// assert_eq!(oci::image!("ghcr.io/org/app").tag(), None);
/// ```
///
/// Docker would read `"MyOrg/app"` as a registry host named `MyOrg`, which is
/// almost never meant, so that is a compile error suggesting `"myorg/app"`, as
/// is upper case in a repository:
///
/// ```compile_fail
/// let _ = constlit::oci::image!("MyOrg/app");
/// ```
pub use constlit_macros::oci_image as image;

/// A platform, checked at compile time, as a [`Platform`].
///
/// `"os/arch"` or `"os/arch/variant"` with Go's `GOOS` and `GOARCH` names, as
/// OCI image indexes use them. Variants: `v5`-`v8` for `arm`; `v8`, `v8.1`-`v8.9`,
/// `v9`, `v9.1`-`v9.5` for `arm64`; `v1`-`v4` for `amd64`; none elsewhere.
/// Common other names (`x86_64`, `aarch64`, `armhf`, `macos`, ...) are compile
/// errors that suggest the Go name.
///
/// ```
/// use constlit::oci::{self, Platform};
/// const PI: Platform = oci::platform!("linux/arm/v7");
/// assert_eq!(PI.arch(), "arm");
/// assert_eq!(PI.variant(), Some("v7"));
/// ```
///
/// ```compile_fail
/// let _ = constlit::oci::platform!("linux/x86_64"); // write "linux/amd64"
/// ```
pub use constlit_macros::oci_platform as platform;
