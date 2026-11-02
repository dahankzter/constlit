//! `unix::signal!`: signal names. Numbers depend on the target, so the parser
//! returns the canonical name and the runtime crate supplies the number.

use crate::suggest;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, quote_spanned};

/// Every signal `signal!` knows, without `SIG`. The runtime crate's table module
/// has one constant per name.
pub const NAMES: [&str; 34] = [
    "HUP", "INT", "QUIT", "ILL", "TRAP", "ABRT", "BUS", "FPE", "KILL", "USR1", "SEGV", "USR2",
    "PIPE", "ALRM", "TERM", "STKFLT", "CHLD", "CONT", "STOP", "TSTP", "TTIN", "TTOU", "URG",
    "XCPU", "XFSZ", "VTALRM", "PROF", "WINCH", "IO", "PWR", "SYS", "EMT", "INFO", "LOST",
];

/// Other names for the same signals.
const ALIASES: [(&str, &str); 3] = [("IOT", "ABRT"), ("POLL", "IO"), ("CLD", "CHLD")];

/// Names of signals 1-31 on most Linux targets, for suggestions only.
const GENERIC_LINUX: [&str; 31] = [
    "HUP", "INT", "QUIT", "ILL", "TRAP", "ABRT", "BUS", "FPE", "KILL", "USR1", "SEGV", "USR2",
    "PIPE", "ALRM", "TERM", "STKFLT", "CHLD", "CONT", "STOP", "TSTP", "TTIN", "TTOU", "URG",
    "XCPU", "XFSZ", "VTALRM", "PROF", "WINCH", "IO", "PWR", "SYS",
];

/// Parses a signal name (`"SIGTERM"` or `"TERM"`) into its canonical name without `SIG`.
pub fn parse_signal(s: &str) -> Result<&'static str, String> {
    if s.is_empty() {
        return Err(r#"empty signal; write a name like "SIGTERM""#.into());
    }
    if s.starts_with(char::is_whitespace) || s.ends_with(char::is_whitespace) {
        return Err("remove the spaces around the literal".into());
    }
    let digits = s.strip_prefix(['-', '+']).unwrap_or(s);
    if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(match s.parse::<usize>() {
            Ok(n @ 1..=31) => format!(
                r#"signal numbers differ between platforms; write the name, e.g. "SIG{}" ({n} on most Linux targets)"#,
                GENERIC_LINUX[n - 1]
            ),
            _ => {
                r#"signal numbers differ between platforms; write the name, like "SIGTERM""#.into()
            }
        });
    }
    // Checks below run on the upper-cased name, so lower-case real-time names and
    // typos get the same help as their upper-case spellings.
    let upper = s.to_ascii_uppercase();
    if upper != s && lookup(&upper).is_some() {
        return Err(format!(r#"signal names are upper case: "{upper}""#));
    }
    let name = upper.strip_prefix("SIG").unwrap_or(&upper);
    if name.starts_with("RTMIN") || name.starts_with("RTMAX") {
        return Err(
            "real-time signal numbers are only known at run time; use libc::SIGRTMIN() instead"
                .into(),
        );
    }
    if let Some(canonical) = lookup(s) {
        return Ok(canonical);
    }
    // Compared without "SIG", so the shared prefix doesn't make far-off names look close.
    let candidates = NAMES.iter().chain(ALIASES.iter().map(|(alias, _)| alias));
    Err(match suggest::closest(name, candidates.copied()) {
        Some(near) => format!(r#"unknown signal "{s}"; did you mean "SIG{near}"?"#),
        None => format!(r#"unknown signal "{s}""#),
    })
}

/// The signal numberings, as `cfg` predicates that must match the tables in
/// `constlit/src/unix/signal.rs`, with the names errors use for them.
fn families() -> [(TokenStream, &'static str); 4] {
    let linux = quote!(any(target_os = "linux", target_os = "android"));
    let mips = quote!(any(
        target_arch = "mips",
        target_arch = "mips64",
        target_arch = "mips32r6",
        target_arch = "mips64r6"
    ));
    let sparc = quote!(any(target_arch = "sparc", target_arch = "sparc64"));
    [
        (
            quote!(all(#linux, not(any(#mips, #sparc)))),
            "Linux and Android",
        ),
        (quote!(all(#linux, #mips)), "Linux on mips"),
        (quote!(all(target_os = "linux", #sparc)), "Linux on sparc"),
        (
            quote!(any(
                target_vendor = "apple",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd",
                target_os = "dragonfly"
            )),
            "macOS, iOS and the BSDs",
        ),
    ]
}

/// Whether each family in [`families`] has the signal `name` (canonical, without `SIG`).
pub fn presence(name: &str) -> [bool; 4] {
    match name {
        "STKFLT" => [true, false, false, false],
        "PWR" => [true, true, true, false],
        "EMT" => [false, true, true, true],
        "INFO" => [false, false, false, true],
        "LOST" => [false, false, true, false],
        _ => [true; 4],
    }
}

/// Whether each family in [`families`] has the alias `alias` (without `SIG`) for its
/// signal. `SIGPOLL` is Linux's: macOS defines it as 7 (a different signal) and the
/// BSDs don't define it.
fn alias_presence(alias: &str) -> [bool; 4] {
    match alias {
        "POLL" => [true, true, true, false],
        _ => [true; 4],
    }
}

/// The expansion of `signal!` for the canonical `name`, written as `written`. The number comes from the
/// runtime crate's table for the user's target. A target without the signal gets a
/// `cfg`-gated `compile_error!` naming its family, at `span` (the literal); the `None` arm can't be reached
/// then, but keeps the constant well-typed.
pub fn expand_signal(name: &'static str, written: &str, span: Span) -> TokenStream {
    let shown = written.strip_prefix("SIG").unwrap_or(written);
    let families = families();
    let all = families.iter().map(|(predicate, _)| predicate);
    let (base, alias) = (presence(name), alias_presence(shown));
    let present: [bool; 4] = core::array::from_fn(|i| base[i] && alias[i]);
    let errors = families
        .iter()
        .zip(present)
        .filter(|(_, present)| !present)
        .map(|((predicate, family), _)| {
            let message = format!("SIG{shown} does not exist on {family}");
            quote_spanned!(span=> #[cfg(#predicate)] ::core::compile_error!(#message);)
        });
    let with: Vec<&TokenStream> = families
        .iter()
        .zip(present)
        .filter(|(_, present)| *present)
        .map(|((predicate, _), _)| predicate)
        .collect();
    let ident = Ident::new(name, Span::call_site());
    let missing = format!("SIG{shown} does not exist on this target");
    let unsupported = quote_spanned!(span=>
        #[cfg(not(any(#(#all),*)))]
        ::core::compile_error!("signal! supports Linux, Android, macOS, iOS and the BSDs");
    );
    quote!({
        #unsupported
        #(#errors)*
        // A named const item, not an inline `const { }` block: `cargo check` (and so
        // rust-analyzer) evaluates the former but leaves the latter to codegen.
        #[cfg(any(#(#with),*))]
        const __CONSTLIT_SIGNAL: ::constlit::unix::Signal = match ::constlit::unix::__signals::#ident {
            ::core::option::Option::Some(n) => ::constlit::unix::Signal::__new_unchecked(n),
            ::core::option::Option::None => ::core::panic!(#missing),
        };
        #[cfg(not(any(#(#with),*)))]
        const __CONSTLIT_SIGNAL: ::constlit::unix::Signal = ::constlit::unix::Signal::__new_unchecked(0);
        __CONSTLIT_SIGNAL
    })
}

/// The canonical name for `s`, with or without `SIG`, resolving aliases.
fn lookup(s: &str) -> Option<&'static str> {
    let name = s.strip_prefix("SIG").unwrap_or(s);
    NAMES.iter().find(|&&n| n == name).copied().or_else(|| {
        ALIASES
            .iter()
            .find(|(alias, _)| *alias == name)
            .map(|(_, canonical)| *canonical)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn err(s: &str) -> String {
        parse_signal(s).expect_err(s)
    }

    #[test]
    fn signal_values() {
        assert_eq!(parse_signal("SIGTERM"), Ok("TERM"));
        assert_eq!(parse_signal("TERM"), Ok("TERM"));
        assert_eq!(parse_signal("SIGIOT"), Ok("ABRT"));
        assert_eq!(parse_signal("SIGPOLL"), Ok("IO"));
        assert_eq!(parse_signal("CLD"), Ok("CHLD"));
        for name in NAMES {
            assert_eq!(parse_signal(name), Ok(name));
            assert_eq!(parse_signal(&format!("SIG{name}")), Ok(name));
        }
        assert_eq!(NAMES.len(), 34);
    }

    #[test]
    fn signal_presence() {
        // generic Linux, Linux on mips, Linux on sparc, BSD; from the plan's table.
        let missing = [
            ("STKFLT", [true, false, false, false]),
            ("PWR", [true, true, true, false]),
            ("EMT", [false, true, true, true]),
            ("INFO", [false, false, false, true]),
            ("LOST", [false, false, true, false]),
        ];
        for name in NAMES {
            let expected = missing
                .iter()
                .find(|(n, _)| *n == name)
                .map_or([true; 4], |(_, p)| *p);
            assert_eq!(presence(name), expected, "{name}");
        }
    }

    #[test]
    fn signal_expansion() {
        let info = expand_signal("INFO", "SIGINFO", Span::call_site()).to_string();
        assert!(
            info.contains(r#"compile_error ! ("SIGINFO does not exist on Linux and Android")"#),
            "{info}"
        );
        assert!(info.contains(r#""signal! supports Linux, Android, macOS, iOS and the BSDs""#));
        let pwr = expand_signal("PWR", "SIGPWR", Span::call_site()).to_string();
        assert!(pwr.contains(r#""SIGPWR does not exist on macOS, iOS and the BSDs""#));
        assert!(!pwr.contains("SIGPWR does not exist on Linux"), "{pwr}");
        // macOS defines SIGPOLL as 7 (SIGEMT's number) and the BSDs not at all, so the
        // Linux alias for SIGIO must not resolve there.
        let poll = expand_signal("IO", "SIGPOLL", Span::call_site()).to_string();
        assert!(
            poll.contains(r#""SIGPOLL does not exist on macOS, iOS and the BSDs""#),
            "{poll}"
        );
        assert!(!poll.contains("SIGPOLL does not exist on Linux"), "{poll}");
        let io = expand_signal("IO", "IO", Span::call_site()).to_string();
        assert!(!io.contains(r#"compile_error ! ("SIGIO"#), "{io}");
        let term = expand_signal("TERM", "SIGTERM", Span::call_site()).to_string();
        assert!(!term.contains(r#"compile_error ! ("SIGTERM"#), "{term}");
    }

    #[test]
    fn signal_errors() {
        let realtime =
            "real-time signal numbers are only known at run time; use libc::SIGRTMIN() instead";
        let cases = [
            ("", r#"empty signal; write a name like "SIGTERM""#),
            ("sigterm", r#"signal names are upper case: "SIGTERM""#),
            ("Term", r#"signal names are upper case: "TERM""#),
            (
                "15",
                r#"signal numbers differ between platforms; write the name, e.g. "SIGTERM" (15 on most Linux targets)"#,
            ),
            (
                "10",
                r#"signal numbers differ between platforms; write the name, e.g. "SIGUSR1" (10 on most Linux targets)"#,
            ),
            (
                "0",
                r#"signal numbers differ between platforms; write the name, like "SIGTERM""#,
            ),
            (
                "64",
                r#"signal numbers differ between platforms; write the name, like "SIGTERM""#,
            ),
            (
                "-9",
                r#"signal numbers differ between platforms; write the name, like "SIGTERM""#,
            ),
            ("SIGRTMIN", realtime),
            ("SIGRTMAX", realtime),
            ("SIGRTMIN+3", realtime),
            ("RTMAX-1", realtime),
            (
                "SIGTREM",
                r#"unknown signal "SIGTREM"; did you mean "SIGTERM"?"#,
            ),
            ("SIGFOO", r#"unknown signal "SIGFOO""#),
            (" SIGTERM", "remove the spaces around the literal"),
            ("sigrtmin", realtime),
            (
                "sigtrem",
                r#"unknown signal "sigtrem"; did you mean "SIGTERM"?"#,
            ),
        ];
        for (input, message) in cases {
            assert_eq!(err(input), message, "input {input:?}");
        }
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn signal_generic_oracle() {
        let libc_numbers = [
            ("HUP", libc::SIGHUP),
            ("INT", libc::SIGINT),
            ("QUIT", libc::SIGQUIT),
            ("ILL", libc::SIGILL),
            ("TRAP", libc::SIGTRAP),
            ("ABRT", libc::SIGABRT),
            ("BUS", libc::SIGBUS),
            ("FPE", libc::SIGFPE),
            ("KILL", libc::SIGKILL),
            ("USR1", libc::SIGUSR1),
            ("SEGV", libc::SIGSEGV),
            ("USR2", libc::SIGUSR2),
            ("PIPE", libc::SIGPIPE),
            ("ALRM", libc::SIGALRM),
            ("TERM", libc::SIGTERM),
            ("STKFLT", libc::SIGSTKFLT),
            ("CHLD", libc::SIGCHLD),
            ("CONT", libc::SIGCONT),
            ("STOP", libc::SIGSTOP),
            ("TSTP", libc::SIGTSTP),
            ("TTIN", libc::SIGTTIN),
            ("TTOU", libc::SIGTTOU),
            ("URG", libc::SIGURG),
            ("XCPU", libc::SIGXCPU),
            ("XFSZ", libc::SIGXFSZ),
            ("VTALRM", libc::SIGVTALRM),
            ("PROF", libc::SIGPROF),
            ("WINCH", libc::SIGWINCH),
            ("IO", libc::SIGIO),
            ("PWR", libc::SIGPWR),
            ("SYS", libc::SIGSYS),
        ];
        for (name, number) in libc_numbers {
            assert_eq!(GENERIC_LINUX[number as usize - 1], name);
        }
        assert_eq!(GENERIC_LINUX.len(), 31);
    }
}
