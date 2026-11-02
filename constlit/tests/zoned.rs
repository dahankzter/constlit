#![cfg(feature = "tz")]

use constlit::{ZonedTimestamp, date, time, ts};

fn hash(t: &ZonedTimestamp) -> u64 {
    use std::hash::{BuildHasher, BuildHasherDefault, DefaultHasher};
    BuildHasherDefault::<DefaultHasher>::default().hash_one(t)
}

#[test]
fn zoned_basics() {
    const Z: ZonedTimestamp = ts!("2026-10-30 14:00 Europe/Stockholm");
    assert_eq!(Z.zone(), "Europe/Stockholm");
    assert_eq!(Z.offset_seconds(), 3600);
    assert_eq!(Z.unix_seconds(), 1_793_365_200);
    assert_eq!(Z.nanosecond(), 0);
    assert_eq!(Z.date(), date!("2026-10-30"));
    assert_eq!(Z.time(), time!("14:00"));
    assert_eq!(Z.timestamp(), ts!("2026-10-30T14:00+01:00"));
    assert_eq!(Z.to_string(), "2026-10-30T14:00:00+01:00 Europe/Stockholm");
    let utc = ts!("2026-10-30 13:00 UTC");
    assert_eq!(utc.to_string(), "2026-10-30T13:00:00+00:00 UTC");
    assert_eq!(Z, utc);
    assert_eq!(hash(&Z), hash(&utc));
    assert!(Z < ts!("2026-10-30 14:00 Europe/London"));
    #[cfg(feature = "jiff")]
    assert_eq!(
        Z.to_jiff_zoned().unwrap(),
        "2026-10-30T14:00+01:00[Europe/Stockholm]"
            .parse::<jiff::Zoned>()
            .unwrap()
    );
}

#[test]
fn zoned_lmt() {
    let paris = ts!("1880-06-01 12:00 Europe/Paris");
    assert_eq!(paris.offset_seconds(), 561);
    assert_eq!(
        paris.to_string(),
        "1880-06-01T12:00:00+00:09:21 Europe/Paris"
    );
    #[cfg(feature = "jiff")]
    assert_eq!(
        paris.to_jiff_zoned().unwrap().timestamp().as_second(),
        paris.unix_seconds()
    );
}
