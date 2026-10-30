use core::time::Duration;

#[test]
fn dur_in_const() {
    const T: Duration = constlit::dur!("1h30m");
    assert_eq!(T, Duration::from_secs(5400));
    const H: Duration = constlit::dur!("250µs");
    assert_eq!(H, Duration::from_micros(250));
}
