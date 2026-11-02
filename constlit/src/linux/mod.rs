//! Linux values: capabilities.
//!
//! ```
//! use constlit::linux::{self, CapSet};
//!
//! const NEEDED: CapSet = linux::caps!("CAP_NET_BIND_SERVICE, CAP_NET_RAW");
//! assert!(NEEDED.contains(linux::cap!("NET_RAW")));
//! assert_eq!(NEEDED.bits(), (1 << 10) | (1 << 13));
//! ```

mod cap;

pub use cap::{CapSet, CapSetIter, Capability};

/// A Linux capability name, checked at compile time, as a [`Capability`].
///
/// Write `"CAP_NET_ADMIN"` or, as Docker and Kubernetes do, `"NET_ADMIN"`, in
/// upper case. The numbers are stable kernel ABI, so they don't depend on the
/// build target. `"ALL"` is a compile error, since the set of all capabilities
/// depends on the kernel.
///
/// ```
/// use constlit::linux::{self, Capability};
/// const BIND: Capability = linux::cap!("CAP_NET_BIND_SERVICE");
/// assert_eq!(BIND.number(), 10);
/// assert_eq!(linux::cap!("NET_BIND_SERVICE"), BIND);
/// ```
///
/// ```compile_fail
/// let _ = constlit::linux::cap!("cap_net_admin");
/// ```
pub use constlit_macros::linux_cap as cap;

/// A comma-separated list of capability names, checked at compile time, as a [`CapSet`].
///
/// Names are separated by `","` or `", "`, each written as for [`cap!`]. Listing
/// a capability twice, even in both spellings, is a compile error.
///
/// ```
/// use constlit::linux::{self, CapSet};
/// const S: CapSet = linux::caps!("CAP_NET_ADMIN, SYS_TIME");
/// assert_eq!(S.len(), 2);
/// assert_eq!(S.to_string(), "CAP_NET_ADMIN, CAP_SYS_TIME");
/// ```
///
/// ```compile_fail
/// let _ = constlit::linux::caps!("NET_ADMIN, CAP_NET_ADMIN");
/// ```
pub use constlit_macros::linux_caps as caps;
