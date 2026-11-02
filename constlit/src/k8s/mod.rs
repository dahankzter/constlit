//! Kubernetes values: resource quantities, object names, labels and label selectors.
//!
//! ```
//! use constlit::k8s::{self, Quantity};
//!
//! const REQUEST: Quantity = k8s::cpu!("250m");
//! const LIMIT: Quantity = k8s::mem!("512Mi");
//! assert_eq!(REQUEST.millis(), 250);
//! assert_eq!(LIMIT.value(), 512 << 20);
//! assert_eq!(LIMIT.to_string(), "512Mi");
//! ```
//!
//! With the `k8s-openapi` feature, a [`Quantity`] converts into k8s-openapi's
//! `Quantity` for use in Pod specs and other API objects. constlit enables no
//! Kubernetes version; like any k8s-openapi user, your crate enables one of
//! k8s-openapi's `v1_*` features (or sets `K8S_OPENAPI_ENABLED_VERSION` for
//! commands like `cargo check`), or the build stops with k8s-openapi's error.

mod quantity;

pub use quantity::Quantity;

/// A Kubernetes quantity, checked at compile time, as a [`Quantity`].
///
/// The grammar is Kubernetes' own: a number with an optional sign and decimal
/// point, then a decimal suffix (`n`, `u`, `m`, `k`, `M`, `G`, `T`, `P`, `E`), a
/// binary suffix (`Ki` ... `Ei`) or an exponent (`e3`, `E-2`). `E` alone is
/// exa; `E` followed by digits is an exponent.
///
/// Where Kubernetes would silently round or cap, this is a compile error: a
/// value finer than `1m`, or beyond 2^63 - 1 units.
///
/// ```
/// use constlit::k8s::{self, Quantity};
/// const Q: Quantity = k8s::quantity!("1.5Gi");
/// assert_eq!(Q.value(), 1_610_612_736);
/// assert_eq!(k8s::quantity!("1e3"), k8s::quantity!("1k"));
/// ```
///
/// ```compile_fail
/// let _ = constlit::k8s::quantity!("0.1m"); // finer than 1m
/// ```
pub use constlit_macros::k8s_quantity as quantity;

/// A CPU amount, checked at compile time, as a [`Quantity`].
///
/// As [`quantity!`], but negative values and binary suffixes (`"2Gi"`) are errors:
/// CPU is counted in cores.
///
/// ```
/// use constlit::k8s::{self, Quantity};
/// const REQUEST: Quantity = constlit::k8s::cpu!("500m");
/// assert_eq!(REQUEST.millis(), 500);
/// ```
///
/// ```compile_fail
/// let _ = constlit::k8s::cpu!("2Gi");
/// ```
pub use constlit_macros::k8s_cpu as cpu;

/// A memory or storage amount, checked at compile time, as a [`Quantity`].
///
/// As [`quantity!`], but the value must be a whole, non-negative number of
/// bytes, and the suffixes `n`, `u` and `m` are errors: `"500m"` is half a byte,
/// and almost always meant `"500Mi"`.
///
/// ```
/// use constlit::k8s::{self, Quantity};
/// const LIMIT: Quantity = k8s::mem!("1.5Gi");
/// assert_eq!(LIMIT.value(), 1_610_612_736);
/// ```
///
/// ```compile_fail
/// let _ = constlit::k8s::mem!("500m");
/// ```
pub use constlit_macros::k8s_mem as mem;

/// An object name that must be an RFC 1123 label, checked at compile time, as a
/// `&'static str`: namespaces and most other object names.
///
/// 1-63 characters of lower-case letters, digits and `-`, starting and ending
/// with a letter or digit.
///
/// ```
/// const NAMESPACE: &str = constlit::k8s::dns_label!("kube-system");
/// assert_eq!(NAMESPACE, "kube-system");
/// ```
///
/// ```compile_fail
/// let _ = constlit::k8s::dns_label!("My-App"); // names are lower case
/// ```
pub use constlit_macros::k8s_dns_label as dns_label;

/// An object name that must be an RFC 1035 label, checked at compile time, as a
/// `&'static str`: Service names, for example.
///
/// As [`dns_label!`], but the first character must be a letter.
///
/// ```
/// const SERVICE: &str = constlit::k8s::dns1035_label!("web");
/// ```
///
/// ```compile_fail
/// let _ = constlit::k8s::dns1035_label!("1web");
/// ```
pub use constlit_macros::k8s_dns1035_label as dns1035_label;

/// An object name that must be an RFC 1123 subdomain, checked at compile time, as
/// a `&'static str`: ConfigMaps, Secrets and most other named objects.
///
/// At most 253 characters, made of dot-separated parts that each follow
/// [`dns_label!`]'s character rules.
///
/// ```
/// const CONFIG: &str = constlit::k8s::dns_subdomain!("app.config.v1");
/// ```
///
/// ```compile_fail
/// let _ = constlit::k8s::dns_subdomain!("app..config");
/// ```
pub use constlit_macros::k8s_dns_subdomain as dns_subdomain;

/// A label (or annotation) key, checked at compile time, as a `&'static str`.
///
/// An optional prefix, which must be a DNS subdomain, and `/`, then a name of
/// 1-63 letters, digits, `-`, `_` and `.`, starting and ending with a letter or
/// digit. The name may use upper case; the prefix may not.
///
/// ```
/// const KEY: &str = constlit::k8s::label_key!("app.kubernetes.io/name");
/// ```
///
/// ```compile_fail
/// let _ = constlit::k8s::label_key!("Example.com/name");
/// ```
pub use constlit_macros::k8s_label_key as label_key;

/// A label value, checked at compile time, as a `&'static str`.
///
/// Empty, or 1-63 letters, digits, `-`, `_` and `.`, starting and ending with a
/// letter or digit.
///
/// ```
/// const VALUE: &str = constlit::k8s::label_value!("Web_App-1");
/// assert_eq!(constlit::k8s::label_value!(""), "");
/// ```
///
/// ```compile_fail
/// let _ = constlit::k8s::label_value!("-web");
/// ```
pub use constlit_macros::k8s_label_value as label_value;

/// A label selector, checked at compile time, as a `&'static str`.
///
/// Kubernetes' syntax: comma-separated requirements, each `key`, `!key`,
/// `key=value` (or `==`, `!=`), `key in (v1, v2)`, `key notin (v1, v2)`, or
/// `key > 5` / `key < 5`. Keys and values follow [`label_key!`] and
/// [`label_value!`]; whitespace around tokens is allowed.
///
/// An empty selector is a compile error: it matches everything, which deserves
/// your API's explicit match-all form. So is an empty value in a set
/// (`"app in (a,)"`), which Kubernetes would read as the value `""`.
///
/// ```
/// const SELECTOR: &str = constlit::k8s::selector!("app=web,env in (prod, staging)");
/// ```
///
/// ```compile_fail
/// let _ = constlit::k8s::selector!("app=-web");
/// ```
pub use constlit_macros::k8s_selector as selector;
