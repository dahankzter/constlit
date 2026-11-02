//! [`Digest`], the value of `oci::digest!`.

use core::fmt;

/// A digest algorithm the OCI image spec registers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DigestAlgorithm {
    /// SHA-256: 32 bytes, written as 64 hex digits.
    Sha256,
    /// SHA-512: 64 bytes, written as 128 hex digits.
    Sha512,
}

impl DigestAlgorithm {
    /// The name in a digest string: `"sha256"` or `"sha512"`.
    pub const fn name(self) -> &'static str {
        match self {
            DigestAlgorithm::Sha256 => "sha256",
            DigestAlgorithm::Sha512 => "sha512",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Bytes {
    Sha256([u8; 32]),
    Sha512([u8; 64]),
}

/// An OCI content digest, usually written with [`digest!`](crate::oci::digest).
///
/// The hex digits are decoded at compile time. `Display` writes
/// `<algorithm>:<lower-case hex>`, the form OCI registries use.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Digest(Bytes);

impl Digest {
    #[doc(hidden)]
    pub const fn __sha256(bytes: [u8; 32]) -> Self {
        Self(Bytes::Sha256(bytes))
    }

    #[doc(hidden)]
    pub const fn __sha512(bytes: [u8; 64]) -> Self {
        Self(Bytes::Sha512(bytes))
    }

    /// The algorithm.
    pub const fn algorithm(&self) -> DigestAlgorithm {
        match self.0 {
            Bytes::Sha256(_) => DigestAlgorithm::Sha256,
            Bytes::Sha512(_) => DigestAlgorithm::Sha512,
        }
    }

    /// The digest's bytes: 32 for SHA-256, 64 for SHA-512.
    pub const fn as_bytes(&self) -> &[u8] {
        match &self.0 {
            Bytes::Sha256(bytes) => bytes,
            Bytes::Sha512(bytes) => bytes,
        }
    }
}

impl fmt::Display for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.algorithm().name())?;
        f.write_str(":")?;
        for byte in self.as_bytes() {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}
