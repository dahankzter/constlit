use constlit::systemd;
use core::time::Duration;

#[test]
fn timespan_values() {
    const T: Duration = systemd::timespan!("1h30min");
    assert_eq!(T, Duration::from_secs(5400));
    assert_eq!(systemd::timespan!("1.5h"), T);
    assert_eq!(systemd::timespan!("500ms"), Duration::from_millis(500));
    assert_eq!(systemd::timespan!("1us"), Duration::from_micros(1));
    assert_eq!(systemd::timespan!("1month"), Duration::from_secs(2_629_800));
    static S: Duration = systemd::timespan!("2 weeks");
    assert_eq!(S, Duration::from_secs(1_209_600));
}

#[test]
fn unit_const() {
    const UNIT: &str = systemd::unit!("getty@tty1.service");
    assert_eq!(UNIT, "getty@tty1.service");
    assert_eq!(systemd::unit!("getty@.service"), "getty@.service");
}
