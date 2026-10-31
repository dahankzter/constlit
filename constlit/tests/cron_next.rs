#![cfg(all(feature = "jiff", feature = "chrono"))]

use constlit::cron;
use jiff::Zoned;
use jiff::civil::{DateTime, date};
use jiff::tz::TimeZone;

fn civil(s: &str) -> DateTime {
    s.parse().unwrap()
}

fn zoned(s: &str) -> Zoned {
    s.parse().unwrap()
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

fn splitmix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// One random field and its mask: `*` or 1-3 items from `min..=max`.
/// `day_field` leaves out `*`-prefixed items other than a bare `*`, where croner's
/// day rule differs; `matches_vixie` pins those.
fn field(state: &mut u64, min: u64, max: u64, day_field: bool) -> (String, u64) {
    let range =
        |a: u64, b: u64, step: u64| (a..=b).step_by(step as usize).fold(0, |m, v| m | 1 << v);
    if splitmix(state).is_multiple_of(4) {
        return ("*".into(), range(min, max, 1));
    }
    let span = max - min + 1;
    let mut r = |n: u64| splitmix(state) % n;
    let mut mask = 0;
    let items: Vec<String> = (0..1 + r(3))
        .map(|_| {
            let a = min + r(span);
            let b = a + r(max - a + 1);
            let step = 1 + r(span.min(15));
            let (text, bits) = match (r(5), day_field) {
                (0, _) => (a.to_string(), range(a, a, 1)),
                (1, _) => (format!("{a}-{b}"), range(a, b, 1)),
                (2, _) => (format!("{a}-{b}/{step}"), range(a, b, step)),
                (3, false) => (format!("*/{step}"), range(min, max, step)),
                // croner rejects `a/n`; `a-max/n` means the same and `cron_values` pins `a/n`.
                (_, false) => (format!("{a}-{max}/{step}"), range(a, max, step)),
                (_, true) => (format!("{a},{b}"), range(a, a, 1) | range(b, b, 1)),
            };
            mask |= bits;
            text
        })
        .collect();
    (items.join(","), mask)
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
