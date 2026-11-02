#![cfg(all(feature = "jiff", feature = "chrono"))]

use constlit::{CronSpec, cron};
use jiff::Zoned;
use jiff::civil::{DateTime, date};
use jiff::tz::TimeZone;

#[path = "support/cron_gen.rs"]
mod cron_gen;
use cron_gen::{field, splitmix};

fn civil(s: &str) -> DateTime {
    s.parse().unwrap()
}

fn zoned(s: &str) -> Zoned {
    s.parse().unwrap()
}

/// The next `n` runs of `spec` after `from`, as jiff writes them.
fn runs(spec: CronSpec, from: &str, n: usize) -> Vec<String> {
    let mut t = zoned(from);
    (0..n)
        .map(|_| {
            t = spec.next_after(&t).unwrap();
            t.to_string()
        })
        .collect()
}

#[test]
fn next_after_strictly_after() {
    let ten = cron!("0 10 * * *");
    let next_day = civil("2026-10-31T10:00:00");
    assert_eq!(
        ten.next_after(&civil("2026-10-30T10:00:30")),
        Some(next_day)
    );
    assert_eq!(
        ten.next_after(&civil("2026-10-30T10:00:00")),
        Some(next_day)
    );
    assert_eq!(
        ten.next_after(&civil("2026-10-30T09:59:59.999")),
        Some(civil("2026-10-30T10:00:00"))
    );
    assert_eq!(
        cron!("0 0 30 2 *").next_after(&civil("2026-10-30T09:00")),
        None
    );
}

#[test]
fn dst_gap_jiff() {
    let spec = cron!("30 2 * * *");
    let first = spec
        .next_after(&zoned("2026-03-28T12:00[Europe/Stockholm]"))
        .unwrap();
    assert_eq!(first, zoned("2026-03-29T03:00+02:00[Europe/Stockholm]"));
    let second = spec.next_after(&first).unwrap();
    assert_eq!(second, zoned("2026-03-30T02:30+02:00[Europe/Stockholm]"));
}

/// As in Vixie cron, schedules whose minute or hour field starts with `*` skip the
/// minutes the clocks jump over, unless made with `catch_up_gaps`.
#[test]
fn dst_gap_wildcard_jiff() {
    let at = |s: &str| format!("2026-03-29T{s}[Europe/Stockholm]");
    let day_before = "2026-03-28T12:00[Europe/Stockholm]";
    assert_eq!(
        runs(cron!("30 * * * *"), &at("01:40+01:00"), 2),
        [at("03:30:00+02:00"), at("04:30:00+02:00")]
    );
    assert_eq!(
        runs(cron!("*/5 * * * *"), &at("01:55+01:00"), 1),
        [at("03:00:00+02:00")]
    );
    assert_eq!(
        runs(cron!("* 2 * * *"), day_before, 1),
        ["2026-03-30T02:00:00+02:00[Europe/Stockholm]"]
    );

    const CATCH_UP: CronSpec = cron!("30 * * * *").catch_up_gaps();
    assert_eq!(
        runs(CATCH_UP, &at("01:40+01:00"), 2),
        [at("03:00:00+02:00"), at("03:30:00+02:00")]
    );
    assert_eq!(
        runs(cron!("* 2 * * *").catch_up_gaps(), day_before, 2),
        [
            at("03:00:00+02:00"),
            "2026-03-30T02:00:00+02:00[Europe/Stockholm]".to_string(),
        ]
    );
    assert_ne!(cron!("30 * * * *"), CATCH_UP);
    // Fixed schedules already catch up.
    assert_eq!(
        runs(cron!("30 2 * * *").catch_up_gaps(), day_before, 2),
        runs(cron!("30 2 * * *"), day_before, 2)
    );
    // The repeated hour is unaffected.
    let fold = "2026-10-25T02:52+02:00[Europe/Stockholm]";
    assert_eq!(
        runs(cron!("*/5 * * * *").catch_up_gaps(), fold, 3),
        runs(cron!("*/5 * * * *"), fold, 3)
    );
}

#[test]
fn dst_overlap_jiff() {
    let spec = cron!("30 2 * * *");
    let first = spec
        .next_after(&zoned("2026-10-24T12:00[Europe/Stockholm]"))
        .unwrap();
    assert_eq!(first, zoned("2026-10-25T02:30+02:00[Europe/Stockholm]"));
    let second = spec.next_after(&first).unwrap();
    assert_eq!(second, zoned("2026-10-26T02:30+01:00[Europe/Stockholm]"));
    // From inside the repeated hour, the job doesn't run a second time that night.
    let inside = zoned("2026-10-25T02:10+01:00[Europe/Stockholm]");
    assert_eq!(spec.next_after(&inside).unwrap(), second);
}

/// Schedules whose minute or hour field starts with `*` run on real time, as in
/// Vixie cron: when the clocks go back they fire through the repeated hour too.
#[test]
fn dst_overlap_wildcard_jiff() {
    let at = |s: &str| format!("2026-10-25T{s}[Europe/Stockholm]");
    assert_eq!(
        runs(cron!("*/5 * * * *"), &at("02:52+02:00"), 4),
        [
            at("02:55:00+02:00"),
            at("02:00:00+01:00"),
            at("02:05:00+01:00"),
            at("02:10:00+01:00"),
        ]
    );
    assert_eq!(
        runs(cron!("*/5 * * * *"), &at("02:57:30+02:00"), 1),
        [at("02:00:00+01:00")]
    );
    assert_eq!(
        runs(cron!("*/5 * * * *"), &at("02:55+01:00"), 1),
        [at("03:00:00+01:00")]
    );
    assert_eq!(
        runs(cron!("30 * * * *"), &at("02:40+02:00"), 2),
        [at("02:30:00+01:00"), at("03:30:00+01:00")]
    );
    assert_eq!(
        runs(cron!("*/30 2 * * *"), &at("02:40+02:00"), 3),
        [
            at("02:00:00+01:00"),
            at("02:30:00+01:00"),
            "2026-10-26T02:00:00+01:00[Europe/Stockholm]".to_string(),
        ]
    );
    assert_eq!(
        runs(cron!("@hourly"), &at("01:30+02:00"), 3),
        [
            at("02:00:00+02:00"),
            at("02:00:00+01:00"),
            at("03:00:00+01:00")
        ]
    );
    // A fixed minute and hour still runs once, even with a step or a list.
    assert_eq!(
        runs(cron!("0,30 2 * * *"), &at("02:40+02:00"), 1),
        ["2026-10-26T02:00:00+01:00[Europe/Stockholm]".to_string()]
    );
}

#[test]
fn dst_chrono() {
    use chrono::TimeZone as _;
    use chrono_tz::Europe::Stockholm;
    let spec = cron!("30 2 * * *");
    let at = |y, mo, d, h, mi| {
        Stockholm
            .with_ymd_and_hms(y, mo, d, h, mi, 0)
            .earliest()
            .unwrap()
    };
    let to_jiff = |t: chrono::DateTime<chrono_tz::Tz>| t.to_rfc3339();

    let first = spec.next_after(&at(2026, 3, 28, 12, 0)).unwrap();
    assert_eq!(to_jiff(first), "2026-03-29T03:00:00+02:00");
    assert_eq!(
        to_jiff(spec.next_after(&first).unwrap()),
        "2026-03-30T02:30:00+02:00"
    );

    let first = spec.next_after(&at(2026, 10, 24, 12, 0)).unwrap();
    assert_eq!(to_jiff(first), "2026-10-25T02:30:00+02:00");
    assert_eq!(
        to_jiff(spec.next_after(&first).unwrap()),
        "2026-10-26T02:30:00+01:00"
    );

    let half_past = cron!("30 * * * *");
    let skipped = at(2026, 3, 29, 1, 40);
    assert_eq!(
        to_jiff(half_past.next_after(&skipped).unwrap()),
        "2026-03-29T03:30:00+02:00"
    );
    assert_eq!(
        to_jiff(half_past.catch_up_gaps().next_after(&skipped).unwrap()),
        "2026-03-29T03:00:00+02:00"
    );

    let quarter = cron!("*/15 * * * *");
    let first = quarter.next_after(&at(2026, 10, 25, 2, 50)).unwrap();
    assert_eq!(to_jiff(first), "2026-10-25T02:00:00+01:00");
    assert_eq!(
        to_jiff(quarter.next_after(&first).unwrap()),
        "2026-10-25T02:15:00+01:00"
    );

    let naive = chrono::NaiveDate::from_ymd_opt(2026, 10, 30)
        .unwrap()
        .and_hms_opt(10, 0, 30)
        .unwrap();
    let next = cron!("0 10 * * *").next_after(&naive).unwrap();
    assert_eq!(next.to_string(), "2026-10-31 10:00:00");
}

#[test]
fn zoned_utc_matches_civil() {
    let spec = cron!("*/7 3 * * MON");
    let start = date(2026, 10, 30).at(5, 6, 7, 0);
    let civil_next = spec.next_after(&start).unwrap();
    let zoned_next = spec
        .next_after(&start.to_zoned(TimeZone::UTC).unwrap())
        .unwrap();
    assert_eq!(zoned_next.datetime(), civil_next);
}

#[test]
fn cron_oracle() {
    let mut state = 17u64;
    for _ in 0..2_000 {
        let fields = [
            field(&mut state, 0, 59, false),
            field(&mut state, 0, 23, false),
            field(&mut state, 1, 31, true),
            field(&mut state, 1, 12, false),
            field(&mut state, 0, 6, true),
        ];
        let expr = fields
            .iter()
            .map(|(text, _)| text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        let source: &'static str = Box::leak(expr.clone().into_boxed_str());
        let mask = |i: usize| fields[i].1;
        let ours = constlit::CronSpec::__new_unchecked(
            source,
            mask(0),
            mask(1) as u32,
            mask(2) as u32,
            mask(3) as u16,
            mask(4) as u8,
            fields[2].0 == "*",
            fields[4].0 == "*",
            fields[0].0.starts_with('*') || fields[1].0.starts_with('*'),
        );
        let theirs: croner::Cron = expr.parse().unwrap_or_else(|e| panic!("{expr}: {e}"));
        let seconds = (splitmix(&mut state) % 4_102_444_800) as i64;
        let mut start = jiff::Timestamp::from_second(seconds)
            .unwrap()
            .to_zoned(TimeZone::UTC)
            .datetime();
        for _ in 0..3 {
            let a = ours.next_after(&start);
            let b = theirs.find_next_occurrence(&start, false).ok();
            assert_eq!(a, b, "{expr} after {start}");
            match a {
                Some(next) => start = next,
                None => break,
            }
        }
    }
}
