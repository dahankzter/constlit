use constlit::{TimeOfDay, time};

#[test]
fn time_accessors() {
    const T: TimeOfDay = time!("14:30:05.250");
    assert_eq!(
        (T.hour(), T.minute(), T.second(), T.nanosecond()),
        (14, 30, 5, 250_000_000)
    );
    assert_eq!(T.to_string(), "14:30:05.25");
    assert_eq!(time!("14:30").to_string(), "14:30:00");
    assert_eq!(
        time!("00:00:00.000000001").to_string(),
        "00:00:00.000000001"
    );
    assert!(time!("09:00") < T);
    assert_eq!(TimeOfDay::new(24, 0, 0, 0), None);
    assert_eq!(TimeOfDay::new(0, 60, 0, 0), None);
    assert_eq!(TimeOfDay::new(0, 0, 60, 0), None);
    assert_eq!(TimeOfDay::new(0, 0, 0, 1_000_000_000), None);
    assert_eq!(TimeOfDay::new(14, 30, 5, 250_000_000), Some(T));
}

#[cfg(feature = "jiff")]
fn splitmix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

#[cfg(feature = "jiff")]
#[test]
fn time_jiff_oracle() {
    let mut state = 7u64;
    for i in 0..5_000 {
        let hour = (splitmix(&mut state) % 24) as u8;
        let minute = (splitmix(&mut state) % 60) as u8;
        let second = (splitmix(&mut state) % 60) as u8;
        // Mix whole seconds, short fractions and full nanoseconds.
        let nanosecond = match i % 3 {
            0 => 0,
            1 => (splitmix(&mut state) % 1000) as u32 * 1_000_000,
            _ => (splitmix(&mut state) % 1_000_000_000) as u32,
        };
        let ours = TimeOfDay::new(hour, minute, second, nanosecond).unwrap();
        let theirs =
            jiff::civil::Time::new(hour as i8, minute as i8, second as i8, nanosecond as i32)
                .unwrap();
        assert_eq!(ours.to_jiff(), theirs);
        assert_eq!(ours.to_string(), theirs.to_string());
    }
}

#[cfg(feature = "chrono")]
#[test]
fn time_chrono() {
    assert_eq!(
        time!("23:59:59.5").to_chrono(),
        chrono::NaiveTime::from_hms_milli_opt(23, 59, 59, 500).unwrap()
    );
}
