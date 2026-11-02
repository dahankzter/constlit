//! Unix values: file permission modes and signals.
//!
//! ```
//! use constlit::unix::{self, Mode};
//!
//! const PRIVATE: Mode = unix::mode!("0600");
//! const SCRIPT: Mode = unix::mode!("rwxr-xr-x");
//! assert_eq!(PRIVATE.bits(), 0o600);
//! assert_eq!(SCRIPT.to_string(), "0755");
//! ```

mod mode;
mod signal;

pub use mode::Mode;
pub use signal::Signal;
#[doc(hidden)]
pub use signal::table as __signals;

/// A file permission mode, checked at compile time, as a [`Mode`].
///
/// Three forms, all for the permission bits only (`0o7777` at most):
/// - **octal**, 3 or 4 digits as `chmod` takes them: `"755"`, `"0755"`, `"4755"`;
/// - **`ls` form**, 9 characters: `"rwxr-xr-x"`, with `s`/`S` in the execute
///   positions for setuid and setgid and `t`/`T` for the sticky bit;
/// - **absolute symbolic**, comma-separated `WHO=PERMS` clauses: `"u=rwx,go=rx"`.
///   Classes not named get no bits, so `"u=rwx"` is `0700`.
///
/// ```
/// use constlit::unix::{self, Mode};
/// const M: Mode = unix::mode!("u=rwx,g=rx,o=");
/// assert_eq!(M, unix::mode!("0750"));
/// assert_eq!(M, unix::mode!("rwxr-x---"));
/// assert_eq!(format!("{M:#}"), "rwxr-x---");
/// assert_eq!(unix::mode!("4755").is_setuid(), true);
/// ```
///
/// Relative modes like `"u+x"` change an existing mode instead of defining one,
/// so they are compile errors, as are Rust's `"0o755"` and an `ls` line with
/// its file type (`"-rwxr-xr-x"`):
///
/// ```compile_fail
/// let _ = constlit::unix::mode!("u+x");
/// ```
pub use constlit_macros::unix_mode as mode;

/// A signal name, checked at compile time, as a [`Signal`] numbered for the build target.
///
/// Write `"SIGTERM"` or `"TERM"`, in upper case; `SIGIOT`, `SIGPOLL` and `SIGCLD`
/// are accepted as aliases. Numbers are compile errors, because many differ
/// between platforms (`SIGUSR1` is 10 on x86 Linux, 16 on mips, 30 on macOS).
///
/// The number comes from the build target's table: Linux and Android (generic,
/// mips and sparc numberings), and macOS, iOS and the BSDs. A signal the target
/// doesn't have, like `SIGPWR` on macOS, is a compile error, and so is any other
/// target. Real-time signals aren't covered: their numbers are only known at run
/// time.
///
/// ```
/// # #[cfg(any(target_os = "linux", target_vendor = "apple"))] {
/// use constlit::unix::{self, Signal};
/// const STOP: Signal = unix::signal!("SIGTERM");
/// assert_eq!(STOP.number(), 15);
/// assert_eq!(unix::signal!("SIGIOT").to_string(), "SIGABRT");
/// # }
/// ```
///
/// ```compile_fail
/// let _ = constlit::unix::signal!("15");
/// ```
pub use constlit_macros::unix_signal as signal;
