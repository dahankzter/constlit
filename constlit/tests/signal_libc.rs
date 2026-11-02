//! Static checks that `signal!`'s per-target numbers match libc's. They run on the
//! host and, with `cargo check --tests --target …`, on other targets.
#![allow(dead_code)]

use constlit::unix::__signals as table;

/// One `const` assertion per signal: `table::NAME == Some(libc::SIGNAME)`.
macro_rules! same {
    ($($name:ident)*) => {
        $(
            const _: () = assert!(matches!(table::$name, Some(n) if n == sig::$name));
        )*
    };
}

// libc 0.2.191 gives mips64 Linux the generic signal numbers (its glibc sysdeps
// list names mips twice and never mips64), so libc is no oracle there; constlit's
// mips table follows the kernel and the signal_tables unit test pins it.
#[cfg(all(
    any(
        target_os = "linux",
        target_os = "android",
        target_vendor = "apple",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "dragonfly"
    ),
    not(any(target_arch = "mips64", target_arch = "mips64r6"))
))]
mod checks {
    use super::*;

    /// libc's numbers under the table's names.
    #[allow(non_upper_case_globals)]
    mod sig {
        #[cfg(any(target_os = "linux", target_os = "android"))]
        pub use libc::SIGPWR as PWR;
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
        pub use libc::SIGSTKFLT as STKFLT;
        pub use libc::{
            SIGABRT as ABRT, SIGALRM as ALRM, SIGBUS as BUS, SIGCHLD as CHLD, SIGCONT as CONT,
            SIGFPE as FPE, SIGHUP as HUP, SIGILL as ILL, SIGINT as INT, SIGIO as IO,
            SIGKILL as KILL, SIGPIPE as PIPE, SIGPROF as PROF, SIGQUIT as QUIT, SIGSEGV as SEGV,
            SIGSTOP as STOP, SIGSYS as SYS, SIGTERM as TERM, SIGTRAP as TRAP, SIGTSTP as TSTP,
            SIGTTIN as TTIN, SIGTTOU as TTOU, SIGURG as URG, SIGUSR1 as USR1, SIGUSR2 as USR2,
            SIGVTALRM as VTALRM, SIGWINCH as WINCH, SIGXCPU as XCPU, SIGXFSZ as XFSZ,
        };
        #[cfg(not(any(target_os = "linux", target_os = "android")))]
        pub use libc::{SIGEMT as EMT, SIGINFO as INFO};
    }

    same! {
        HUP INT QUIT ILL TRAP ABRT BUS FPE KILL USR1 SEGV USR2 PIPE ALRM TERM CHLD CONT
        STOP TSTP TTIN TTOU URG XCPU XFSZ VTALRM PROF WINCH IO SYS
    }
    #[cfg(any(target_os = "linux", target_os = "android"))]
    same! { PWR }
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
    same! { STKFLT }
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    same! { EMT INFO }

    const _: () = assert!(table::SUPPORTED);
}
