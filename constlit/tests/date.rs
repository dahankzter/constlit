use constlit::{Date, Weekday, date};

#[test]
fn date_accessors() {
    const D: Date = date!("2026-10-30");
    assert_eq!((D.year(), D.month(), D.day()), (2026, 10, 30));
    assert_eq!(D.weekday(), Weekday::Friday);
    assert_eq!(Weekday::Friday.number_from_monday(), 5);
    assert_eq!(Weekday::Friday.num_days_from_sunday(), 5);
    assert_eq!(Weekday::Sunday.number_from_monday(), 7);
    assert_eq!(Weekday::Sunday.num_days_from_sunday(), 0);
    assert_eq!(D.day_of_year(), 303);
    assert_eq!(D.to_string(), "2026-10-30");
    assert_eq!(date!("0001-01-01").to_string(), "0001-01-01");
    assert!(date!("2026-10-29") < D);
    assert_eq!(Date::new(2026, 2, 30), None);
    assert_eq!(Date::new(0, 1, 1), None);
    assert_eq!(Date::new(10_000, 1, 1), None);
    assert_eq!(Date::new(2026, 10, 30), Some(D));
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
fn date_jiff_oracle() {
    let mut state = 5u64;
    let mut checked = 0;
    while checked < 5_000 {
        let year = 1 + (splitmix(&mut state) % 9999) as i16;
        let month = 1 + (splitmix(&mut state) % 12) as u8;
        let day = 1 + (splitmix(&mut state) % 31) as u8;
        let Some(ours) = Date::new(year, month, day) else {
            continue;
        };
        checked += 1;
        let theirs = jiff::civil::date(year, month as i8, day as i8);
        assert_eq!(ours.to_jiff(), theirs);
        assert_eq!(
            ours.weekday().number_from_monday(),
            theirs.weekday().to_monday_one_offset() as u8,
            "{theirs}"
        );
        assert_eq!(ours.day_of_year(), theirs.day_of_year() as u16, "{theirs}");
        assert_eq!(ours.to_string(), theirs.to_string());
    }
}

#[cfg(feature = "chrono")]
#[test]
fn date_chrono() {
    assert_eq!(
        date!("2024-02-29").to_chrono(),
        chrono::NaiveDate::from_ymd_opt(2024, 2, 29).unwrap()
    );
}
