//! [`Quantity`], the value of `k8s::quantity!`, `cpu!` and `mem!`.

use core::cmp::Ordering;
use core::fmt;
use core::hash::{Hash, Hasher};

/// A Kubernetes resource quantity, usually written with
/// [`quantity!`](crate::k8s::quantity), [`cpu!`](crate::k8s::cpu) or
/// [`mem!`](crate::k8s::mem).
///
/// It keeps the exact value in milli-units and the literal as written.
/// `Display` writes the literal, which Kubernetes accepts unchanged.
/// `Eq`, `Ord` and `Hash` compare the value only, so `quantity!("1Ki")` equals
/// `quantity!("1024")`.
#[derive(Clone, Copy, Debug)]
pub struct Quantity {
    millis: i128,
    text: &'static str,
}

impl Quantity {
    #[doc(hidden)]
    pub const fn __new_unchecked(millis: i128, text: &'static str) -> Self {
        Self { millis, text }
    }

    /// The value in milli-units: `500m` is 500, `1Ki` is 1,024,000.
    pub const fn millis(&self) -> i128 {
        self.millis
    }

    /// The value in whole units, rounded away from zero like Kubernetes'
    /// `Value()`: `500m` is 1, `-1.5` is -2.
    pub const fn value(&self) -> i64 {
        let units = self.millis / 1000;
        let rounded = if self.millis % 1000 == 0 {
            units
        } else if self.millis > 0 {
            units + 1
        } else {
            units - 1
        };
        // The macros keep the magnitude within 2^63 - 1 units.
        rounded as i64
    }

    /// The literal as written.
    pub const fn as_str(&self) -> &'static str {
        self.text
    }
}

impl PartialEq for Quantity {
    fn eq(&self, other: &Self) -> bool {
        self.millis == other.millis
    }
}

impl Eq for Quantity {}

impl PartialOrd for Quantity {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Quantity {
    fn cmp(&self, other: &Self) -> Ordering {
        self.millis.cmp(&other.millis)
    }
}

impl Hash for Quantity {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.millis.hash(state);
    }
}

/// The literal as written, which Kubernetes accepts unchanged.
#[cfg(feature = "k8s-openapi")]
#[cfg_attr(docsrs, doc(cfg(feature = "k8s-openapi")))]
impl From<Quantity> for k8s_openapi::apimachinery::pkg::api::resource::Quantity {
    fn from(quantity: Quantity) -> Self {
        Self(alloc::string::String::from(quantity.text))
    }
}

impl fmt::Display for Quantity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.text)
    }
}
