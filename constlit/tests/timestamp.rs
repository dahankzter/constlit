use constlit::{Timestamp, date, time, ts};

const T: Timestamp = ts!("2026-10-30T15:00:00.25+01:00");

fn hash(t: &Timestamp) -> u64 {
    use std::hash::{BuildHasher, BuildHasherDefault, DefaultHasher};
    BuildHasherDefault::<DefaultHasher>::default().hash_one(t)
}

#[test]
fn timestamp_basics() {
    assert_eq!(T.unix_seconds(), 1_793_368_800);
    assert_eq!(T.nanosecond(), 250_000_000);
    assert_eq!(T.offset_seconds(), 3600);
    assert_eq!(T.date(), date!("2026-10-30"));
    assert_eq!(T.time(), time!("15:00:00.25"));
    assert_eq!(T.to_string(), "2026-10-30T15:00:00.25+01:00");
    assert_eq!(ts!("2026-10-30T14:00Z").to_string(), "2026-10-30T14:00:00Z");
    assert_eq!(
        ts!("2026-10-30T14:00-05:30").to_string(),
        "2026-10-30T14:00:00-05:30"
    );
    assert_eq!(ts!("2026-10-30T14:00Z"), ts!("2026-10-30T15:00+01:00"));
    assert_eq!(
        hash(&ts!("2026-10-30T14:00Z")),
        hash(&ts!("2026-10-30T15:00+01:00"))
    );
    assert!(ts!("2026-10-30T14:00Z") < ts!("2026-10-30T14:00-01:00"));
    assert_eq!(Timestamp::new(0, 1_000_000_000, 0), None);
    assert_eq!(Timestamp::new(0, 0, 86_400), None);
    assert_eq!(Timestamp::new(-62_135_596_801, 0, 0), None);
    assert!(Timestamp::new(-62_135_596_800, 0, 0).is_some());
    assert_eq!(
        Timestamp::new(1_793_368_800, 250_000_000, 3600)
            .unwrap()
            .to_string(),
        T.to_string()
    );
}

#[test]
fn timestamp_before_epoch() {
    let t = ts!("1969-12-31T23:59:59.5Z");
    assert_eq!(t.unix_seconds(), -1);
    assert_eq!(t.nanosecond(), 500_000_000);
    assert_eq!(t.to_string(), "1969-12-31T23:59:59.5Z");
}

#[test]
fn timestamp_extremes() {
    let first = ts!("0001-01-01T00:00+01:00");
    let last = ts!("9999-12-31T23:59-01:00");
    assert_eq!(first.date(), date!("0001-01-01"));
    assert_eq!(last.date(), date!("9999-12-31"));
    assert_eq!(first.to_string(), "0001-01-01T00:00:00+01:00");
    // jiff's range ends about a day before ours.
    #[cfg(feature = "jiff")]
    {
        assert_eq!(
            first.to_jiff(),
            Some(jiff::Timestamp::new(first.unix_seconds(), 0).unwrap())
        );
        assert_eq!(last.to_jiff(), None);
        assert!(ts!("9999-12-30T12:00Z").to_jiff().is_some());
    }
    #[cfg(feature = "chrono")]
    for t in [first, last] {
        assert_eq!(t.to_chrono().timestamp(), t.unix_seconds());
    }
}

#[cfg(feature = "jiff")]
#[test]
fn timestamp_jiff() {
    assert_eq!(
        T.to_jiff(),
        Some(
            "2026-10-30T14:00:00.25Z"
                .parse::<jiff::Timestamp>()
                .unwrap()
        )
    );
}

#[cfg(feature = "chrono")]
#[test]
fn timestamp_chrono() {
    assert_eq!(T.to_chrono().to_rfc3339(), "2026-10-30T15:00:00.250+01:00");
    assert_eq!(
        T.to_chrono_utc().to_rfc3339(),
        "2026-10-30T14:00:00.250+00:00"
    );
}

#[cfg(feature = "std")]
#[test]
fn timestamp_system_time() {
    use std::time::{Duration, UNIX_EPOCH};
    assert_eq!(
        T.to_system_time(),
        UNIX_EPOCH + Duration::new(1_793_368_800, 250_000_000)
    );
    assert_eq!(
        ts!("1969-12-31T23:59:59.5Z").to_system_time(),
        UNIX_EPOCH - Duration::from_millis(500)
    );
}
