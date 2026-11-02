//! Linux values: capabilities, errno values, interface names and device numbers.
//!
//! ```
//! use constlit::linux::{self, CapSet};
//!
//! const NEEDED: CapSet = linux::caps!("CAP_NET_BIND_SERVICE, CAP_NET_RAW");
//! assert!(NEEDED.contains(linux::cap!("NET_RAW")));
//! assert_eq!(NEEDED.bits(), (1 << 10) | (1 << 13));
//! ```

mod cap;
mod dev;
mod errno;

pub use cap::{CapSet, CapSetIter, Capability};
pub use dev::DevNum;
pub use errno::Errno;
#[doc(hidden)]
pub use errno::table as __errnos;

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

/// An errno name, checked at compile time, as an [`Errno`] numbered for the build
/// target.
///
/// Names with their `E`, in upper case; `EWOULDBLOCK` and `ENOTSUP` are accepted as
/// aliases of `EAGAIN` and `EOPNOTSUPP`. Numbers are compile errors, because some
/// differ between architectures (`EDEADLOCK` is 35 on x86, 58 on powerpc).
///
/// The number comes from the build target's table: Linux and Android, with the
/// generic, powerpc, mips and sparc numberings. Any other target is a compile
/// error; the BSDs number errno differently.
///
/// ```
/// # #[cfg(target_os = "linux")] {
/// use constlit::linux::{self, Errno};
/// const MISSING: Errno = linux::errno!("ENOENT");
/// assert_eq!(MISSING.number(), 2);
/// assert_eq!(linux::errno!("EWOULDBLOCK").to_string(), "EAGAIN");
/// # }
/// ```
///
/// ```compile_fail
/// let _ = constlit::linux::errno!("2");
/// ```
pub use constlit_macros::linux_errno as errno;

/// A network interface name, checked at compile time, as a `&'static str`.
///
/// The kernel's rule: 1-15 bytes, not `.` or `..`, and no `/`, `:` or whitespace.
/// Stricter than the kernel, only printable ASCII is allowed, and `%` (which makes
/// a name template like `eth%d`) is an error.
///
/// ```
/// const UPLINK: &str = constlit::linux::ifname!("enp3s0");
/// ```
///
/// ```compile_fail
/// let _ = constlit::linux::ifname!("a-very-long-name0"); // more than 15 bytes
/// ```
pub use constlit_macros::linux_ifname as ifname;

/// A device number, checked at compile time, as a [`DevNum`].
///
/// `"major:minor"` in decimal, within the kernel's limits (major 0-4095, minor
/// 0-1048575), as device cgroup rules and container runtimes write it.
///
/// ```
/// use constlit::linux::{self, DevNum};
/// const SDA: DevNum = linux::dev!("8:0");
/// assert_eq!(SDA.major(), 8);
/// ```
///
/// ```compile_fail
/// let _ = constlit::linux::dev!("4096:0");
/// ```
pub use constlit_macros::linux_dev as dev;
