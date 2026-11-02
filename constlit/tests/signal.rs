// The host test target uses the generic Linux numbering.
#![cfg(all(
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

use constlit::unix::{self, Signal};

#[test]
fn signal_values() {
    const T: Signal = unix::signal!("SIGTERM");
    assert_eq!(T.number(), 15);
    assert_eq!(T.name(), "SIGTERM");
    assert_eq!(T.to_string(), "SIGTERM");
    assert_eq!(unix::signal!("TERM"), T);
    assert_eq!(unix::signal!("SIGIOT"), unix::signal!("SIGABRT"));
    assert_eq!(unix::signal!("SIGIOT").to_string(), "SIGABRT");
    assert_eq!(unix::signal!("SIGPOLL").name(), "SIGIO");
    assert_eq!(Signal::from_number(10).unwrap().name(), "SIGUSR1");
    assert_eq!(Signal::from_number(15), Some(T));
    assert_eq!(Signal::from_number(0), None);
    assert_eq!(Signal::from_number(32), None);
    assert!(unix::signal!("SIGHUP") < T);
}

#[test]
fn signal_contexts() {
    let s = unix::signal!("SIGHUP");
    assert_eq!(s.number(), 1);
    static S: Signal = unix::signal!("SIGKILL");
    assert_eq!(S.number(), 9);
    let caught = match Signal::from_number(2).unwrap() {
        x if x == unix::signal!("SIGINT") => "interrupt",
        _ => "other",
    };
    assert_eq!(caught, "interrupt");
}
