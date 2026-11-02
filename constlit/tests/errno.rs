// The host test target uses the generic Linux numbering.
#![cfg(all(
    any(target_os = "linux", target_os = "android"),
    not(any(
        target_arch = "mips",
        target_arch = "mips64",
        target_arch = "mips32r6",
        target_arch = "mips64r6",
        target_arch = "sparc",
        target_arch = "sparc64",
        target_arch = "powerpc",
        target_arch = "powerpc64"
    ))
))]

use constlit::linux::{self, Errno};

#[test]
fn errno_values() {
    const E: Errno = linux::errno!("ENOENT");
    assert_eq!(E.number(), 2);
    assert_eq!(E.name(), "ENOENT");
    assert_eq!(E.to_string(), "ENOENT");
    assert_eq!(linux::errno!("EWOULDBLOCK"), linux::errno!("EAGAIN"));
    assert_eq!(linux::errno!("EWOULDBLOCK").to_string(), "EAGAIN");
    assert_eq!(linux::errno!("ENOTSUP").name(), "EOPNOTSUPP");
    assert_eq!(linux::errno!("EDEADLOCK"), linux::errno!("EDEADLK"));
    assert_eq!(Errno::from_number(2), Some(E));
    assert_eq!(Errno::from_number(0), None);
    assert_eq!(Errno::from_number(5000), None);
    let e = linux::errno!("EACCES");
    assert_eq!(e.number(), 13);
    static S: Errno = linux::errno!("EINTR");
    assert_eq!(S.number(), 4);
}
