use constlit::unix::{self, Mode};

#[test]
fn mode_accessors() {
    const M: Mode = unix::mode!("rwsr-x---");
    assert_eq!(M.bits(), 0o4750);
    assert!(M.is_setuid());
    assert!(!M.is_setgid());
    assert!(!M.is_sticky());
    assert_eq!((M.user(), M.group(), M.other()), (7, 5, 0));
    assert_eq!(M.to_string(), "4750");
    assert_eq!(format!("{M:#}"), "rwsr-x---");
    assert_eq!(unix::mode!("644").to_string(), "0644");
    assert_eq!(format!("{:#}", unix::mode!("1777")), "rwxrwxrwt");
    assert_eq!(format!("{:#}", unix::mode!("u=rw,g=,o=t")), "rw------T");
    assert_eq!(u32::from(M), 0o4750);
    assert_eq!(Mode::new(0o4750), Some(M));
    assert_eq!(unix::mode!("u=rwx,go=rx"), unix::mode!("0755"));

    static S: Mode = unix::mode!("0600");
    assert_eq!(S.bits(), 0o600);
}

#[cfg(all(feature = "std", unix))]
#[test]
fn mode_permissions() {
    use std::os::unix::fs::PermissionsExt;
    const M: Mode = unix::mode!("rwsr-x---");
    assert_eq!(M.to_permissions().mode() & 0o7777, 0o4750);
}
