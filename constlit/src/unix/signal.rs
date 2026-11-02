//! [`Signal`], the value of `unix::signal!`.
//!
//! Signal numbers differ between platforms. The tables below hold each family's
//! numbering, and [`TABLE`] picks the build target's. `signal!` expands to a named
//! const item in the user's crate that reads [`table`], so the number is the
//! target's; a target without the signal gets a `cfg`-gated `compile_error!` from
//! the macro instead.

use core::fmt;

type Table = [(&'static str, i32)];

/// x86, x86_64, arm, aarch64, riscv, powerpc, s390x, loongarch and other Linux
/// targets using the asm-generic numbering.
// Only the build target's table is used outside tests.
#[allow(dead_code)]
const GENERIC_LINUX: [(&str, i32); 31] = [
    ("SIGHUP", 1),
    ("SIGINT", 2),
    ("SIGQUIT", 3),
    ("SIGILL", 4),
    ("SIGTRAP", 5),
    ("SIGABRT", 6),
    ("SIGBUS", 7),
    ("SIGFPE", 8),
    ("SIGKILL", 9),
    ("SIGUSR1", 10),
    ("SIGSEGV", 11),
    ("SIGUSR2", 12),
    ("SIGPIPE", 13),
    ("SIGALRM", 14),
    ("SIGTERM", 15),
    ("SIGSTKFLT", 16),
    ("SIGCHLD", 17),
    ("SIGCONT", 18),
    ("SIGSTOP", 19),
    ("SIGTSTP", 20),
    ("SIGTTIN", 21),
    ("SIGTTOU", 22),
    ("SIGURG", 23),
    ("SIGXCPU", 24),
    ("SIGXFSZ", 25),
    ("SIGVTALRM", 26),
    ("SIGPROF", 27),
    ("SIGWINCH", 28),
    ("SIGIO", 29),
    ("SIGPWR", 30),
    ("SIGSYS", 31),
];

/// Linux on mips.
// Only the build target's table is used outside tests.
#[allow(dead_code)]
const MIPS_LINUX: [(&str, i32); 31] = [
    ("SIGHUP", 1),
    ("SIGINT", 2),
    ("SIGQUIT", 3),
    ("SIGILL", 4),
    ("SIGTRAP", 5),
    ("SIGABRT", 6),
    ("SIGEMT", 7),
    ("SIGFPE", 8),
    ("SIGKILL", 9),
    ("SIGBUS", 10),
    ("SIGSEGV", 11),
    ("SIGSYS", 12),
    ("SIGPIPE", 13),
    ("SIGALRM", 14),
    ("SIGTERM", 15),
    ("SIGUSR1", 16),
    ("SIGUSR2", 17),
    ("SIGCHLD", 18),
    ("SIGPWR", 19),
    ("SIGWINCH", 20),
    ("SIGURG", 21),
    ("SIGIO", 22),
    ("SIGSTOP", 23),
    ("SIGTSTP", 24),
    ("SIGCONT", 25),
    ("SIGTTIN", 26),
    ("SIGTTOU", 27),
    ("SIGVTALRM", 28),
    ("SIGPROF", 29),
    ("SIGXCPU", 30),
    ("SIGXFSZ", 31),
];

/// Linux on sparc. `SIGLOST` is the same signal as `SIGPWR`, which names it.
// Only the build target's table is used outside tests.
#[allow(dead_code)]
const SPARC_LINUX: [(&str, i32); 32] = [
    ("SIGHUP", 1),
    ("SIGINT", 2),
    ("SIGQUIT", 3),
    ("SIGILL", 4),
    ("SIGTRAP", 5),
    ("SIGABRT", 6),
    ("SIGEMT", 7),
    ("SIGFPE", 8),
    ("SIGKILL", 9),
    ("SIGBUS", 10),
    ("SIGSEGV", 11),
    ("SIGSYS", 12),
    ("SIGPIPE", 13),
    ("SIGALRM", 14),
    ("SIGTERM", 15),
    ("SIGURG", 16),
    ("SIGSTOP", 17),
    ("SIGTSTP", 18),
    ("SIGCONT", 19),
    ("SIGCHLD", 20),
    ("SIGTTIN", 21),
    ("SIGTTOU", 22),
    ("SIGIO", 23),
    ("SIGXCPU", 24),
    ("SIGXFSZ", 25),
    ("SIGVTALRM", 26),
    ("SIGPROF", 27),
    ("SIGWINCH", 28),
    ("SIGPWR", 29),
    ("SIGLOST", 29),
    ("SIGUSR1", 30),
    ("SIGUSR2", 31),
];

/// macOS, iOS and the other Apple platforms, FreeBSD, NetBSD, OpenBSD and DragonFly.
// Only the build target's table is used outside tests.
#[allow(dead_code)]
const BSD: [(&str, i32); 31] = [
    ("SIGHUP", 1),
    ("SIGINT", 2),
    ("SIGQUIT", 3),
    ("SIGILL", 4),
    ("SIGTRAP", 5),
    ("SIGABRT", 6),
    ("SIGEMT", 7),
    ("SIGFPE", 8),
    ("SIGKILL", 9),
    ("SIGBUS", 10),
    ("SIGSEGV", 11),
    ("SIGSYS", 12),
    ("SIGPIPE", 13),
    ("SIGALRM", 14),
    ("SIGTERM", 15),
    ("SIGURG", 16),
    ("SIGSTOP", 17),
    ("SIGTSTP", 18),
    ("SIGCONT", 19),
    ("SIGCHLD", 20),
    ("SIGTTIN", 21),
    ("SIGTTOU", 22),
    ("SIGIO", 23),
    ("SIGXCPU", 24),
    ("SIGXFSZ", 25),
    ("SIGVTALRM", 26),
    ("SIGPROF", 27),
    ("SIGWINCH", 28),
    ("SIGINFO", 29),
    ("SIGUSR1", 30),
    ("SIGUSR2", 31),
];

#[cfg(all(
    any(target_os = "linux", target_os = "android"),
    any(
        target_arch = "mips",
        target_arch = "mips64",
        target_arch = "mips32r6",
        target_arch = "mips64r6"
    )
))]
const TABLE: &Table = &MIPS_LINUX;
#[cfg(all(
    target_os = "linux",
    any(target_arch = "sparc", target_arch = "sparc64")
))]
const TABLE: &Table = &SPARC_LINUX;
#[cfg(all(
    any(target_os = "linux", target_os = "android"),
    not(any(
        target_arch = "mips",
        target_arch = "mips64",
        target_arch = "mips32r6",
        target_arch = "mips64r6",
        target_arch = "sparc",
        target_arch = "sparc64"
    ))
))]
const TABLE: &Table = &GENERIC_LINUX;
#[cfg(any(
    target_vendor = "apple",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
const TABLE: &Table = &BSD;
#[cfg(not(any(
    target_os = "linux",
    target_os = "android",
    target_vendor = "apple",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
)))]
const TABLE: &Table = &[];

const fn str_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// The number of `name` (with `SIG`) in `table`.
const fn number_of(table: &Table, name: &str) -> Option<i32> {
    let mut i = 0;
    while i < table.len() {
        if str_eq(table[i].0, name) {
            return Some(table[i].1);
        }
        i += 1;
    }
    None
}

/// The canonical name of `number` in `table`: the first entry with that number.
const fn name_of(table: &Table, number: i32) -> Option<&'static str> {
    let mut i = 0;
    while i < table.len() {
        if table[i].1 == number {
            return Some(table[i].0);
        }
        i += 1;
    }
    None
}

/// The build target's signal numbers, one constant per name `signal!` knows
/// (`None` where the target lacks the signal). Not public API.
pub mod table {
    use super::{TABLE, number_of};

    /// Whether `signal!` has a numbering for this target.
    pub const SUPPORTED: bool = !TABLE.is_empty();

    macro_rules! signals {
        ($($name:ident)*) => {
            $(
                pub const $name: Option<i32> = number_of(TABLE, concat!("SIG", stringify!($name)));
            )*
        };
    }

    signals! {
        HUP INT QUIT ILL TRAP ABRT BUS FPE KILL USR1 SEGV USR2 PIPE ALRM TERM STKFLT CHLD CONT
        STOP TSTP TTIN TTOU URG XCPU XFSZ VTALRM PROF WINCH IO PWR SYS EMT INFO LOST
    }
}

/// A Unix signal with the build target's number, usually written with
/// [`signal!`](crate::unix::signal).
///
/// `Eq`, `Ord` and `Hash` go by number, and `Display` writes the canonical name
/// (`SIGABRT` for `signal!("SIGIOT")`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Signal(i32);

impl Signal {
    /// The signal with this number on the build target, or `None` if it has no
    /// such signal (real-time signals are not covered).
    pub const fn from_number(number: i32) -> Option<Self> {
        match name_of(TABLE, number) {
            Some(_) => Some(Self(number)),
            None => None,
        }
    }

    #[doc(hidden)]
    pub const fn __new_unchecked(number: i32) -> Self {
        Self(number)
    }

    /// The signal's number on the build target.
    pub const fn number(self) -> i32 {
        self.0
    }

    /// The signal's canonical name, like `"SIGTERM"`.
    pub const fn name(self) -> &'static str {
        match name_of(TABLE, self.0) {
            Some(name) => name,
            None => "SIG?",
        }
    }
}

impl fmt::Display for Signal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The plan's signal table: generic, mips, sparc and bsd numbers (0 = absent).
    const ROWS: [(&str, [i32; 4]); 34] = [
        ("SIGHUP", [1, 1, 1, 1]),
        ("SIGINT", [2, 2, 2, 2]),
        ("SIGQUIT", [3, 3, 3, 3]),
        ("SIGILL", [4, 4, 4, 4]),
        ("SIGTRAP", [5, 5, 5, 5]),
        ("SIGABRT", [6, 6, 6, 6]),
        ("SIGBUS", [7, 10, 10, 10]),
        ("SIGFPE", [8, 8, 8, 8]),
        ("SIGKILL", [9, 9, 9, 9]),
        ("SIGUSR1", [10, 16, 30, 30]),
        ("SIGSEGV", [11, 11, 11, 11]),
        ("SIGUSR2", [12, 17, 31, 31]),
        ("SIGPIPE", [13, 13, 13, 13]),
        ("SIGALRM", [14, 14, 14, 14]),
        ("SIGTERM", [15, 15, 15, 15]),
        ("SIGSTKFLT", [16, 0, 0, 0]),
        ("SIGCHLD", [17, 18, 20, 20]),
        ("SIGCONT", [18, 25, 19, 19]),
        ("SIGSTOP", [19, 23, 17, 17]),
        ("SIGTSTP", [20, 24, 18, 18]),
        ("SIGTTIN", [21, 26, 21, 21]),
        ("SIGTTOU", [22, 27, 22, 22]),
        ("SIGURG", [23, 21, 16, 16]),
        ("SIGXCPU", [24, 30, 24, 24]),
        ("SIGXFSZ", [25, 31, 25, 25]),
        ("SIGVTALRM", [26, 28, 26, 26]),
        ("SIGPROF", [27, 29, 27, 27]),
        ("SIGWINCH", [28, 20, 28, 28]),
        ("SIGIO", [29, 22, 23, 23]),
        ("SIGPWR", [30, 19, 29, 0]),
        ("SIGSYS", [31, 12, 12, 12]),
        ("SIGEMT", [0, 7, 7, 7]),
        ("SIGINFO", [0, 0, 0, 29]),
        ("SIGLOST", [0, 0, 29, 0]),
    ];

    #[test]
    fn signal_tables() {
        let tables: [&Table; 4] = [&GENERIC_LINUX, &MIPS_LINUX, &SPARC_LINUX, &BSD];
        for (family, table) in tables.iter().enumerate() {
            for (name, numbers) in ROWS {
                let expected = Some(numbers[family]).filter(|&n| n != 0);
                assert_eq!(number_of(table, name), expected, "{name} in table {family}");
            }
            // Every number 1-31 has exactly one canonical name.
            for n in 1..=31 {
                let name = name_of(table, n).unwrap_or_else(|| panic!("{n} in table {family}"));
                assert_eq!(number_of(table, name), Some(n));
            }
            assert_eq!(table.len(), if family == 2 { 32 } else { 31 });
        }
        assert_eq!(name_of(&SPARC_LINUX, 29), Some("SIGPWR"));
        assert_eq!(number_of(&SPARC_LINUX, "SIGLOST"), Some(29));
    }
}
