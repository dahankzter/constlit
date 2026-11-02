use constlit::linux::{self, DevNum};

#[test]
fn ifname_const() {
    const NIC: &str = linux::ifname!("enp3s0");
    assert_eq!(NIC, "enp3s0");
}

#[test]
fn dev_accessors() {
    const SDA1: DevNum = linux::dev!("8:1");
    assert_eq!((SDA1.major(), SDA1.minor()), (8, 1));
    assert_eq!(SDA1.to_string(), "8:1");
    assert_eq!(DevNum::new(8, 1), Some(SDA1));
    assert_eq!(DevNum::new(4096, 0), None);
    assert_eq!(DevNum::new(0, 1 << 20), None);
    assert_eq!(linux::dev!("4095:1048575").minor(), 1_048_575);
    static S: DevNum = linux::dev!("1:3");
    assert_eq!(S.major(), 1);
}

#[cfg(target_os = "linux")]
#[test]
fn dev_t_matches_libc() {
    for major in (0..=4095u32).step_by(97).chain([4095]) {
        for minor in [0u32, 1, 255, 256, 65535, 1_048_575] {
            let dev = DevNum::new(major, minor).unwrap();
            assert_eq!(
                dev.to_dev_t(),
                libc::makedev(major, minor),
                "{major}:{minor}"
            );
        }
    }
}
