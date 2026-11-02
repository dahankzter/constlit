//! Named time zones for `ts!`, resolved at compile time from jiff's bundled database.

use jiff::civil::DateTime;
use jiff::tz::{AmbiguousOffset, TimeZone, TimeZoneDatabase};

/// The version of the bundled IANA time-zone database.
pub fn tzdb_version() -> &'static str {
    jiff_tzdb::VERSION.unwrap_or("unknown")
}

/// Resolves a local date and time in `zone` to Unix seconds, nanoseconds and the
/// offset in seconds. `written` is the date and time as written, for messages;
/// `offset` is an explicit offset from the literal.
pub fn resolve(
    written: &str,
    (year, month, day): (i16, u8, u8),
    (hour, minute, second, nanosecond): (u8, u8, u8, u32),
    offset: Option<i32>,
    zone: &str,
) -> Result<(i64, u32, i32), String> {
    let tz = lookup(zone)?;
    let civil = DateTime::new(
        year,
        month as i8,
        day as i8,
        hour as i8,
        minute as i8,
        second as i8,
        nanosecond as i32,
    )
    .map_err(|e| e.to_string())?;
    let chosen = match (tz.to_ambiguous_timestamp(civil).offset(), offset) {
        (AmbiguousOffset::Unambiguous { offset: actual }, None) => actual,
        (AmbiguousOffset::Unambiguous { offset: actual }, Some(given)) => {
            if actual.seconds() != given {
                let actual = show(actual.seconds());
                return Err(format!(
                    r#"{zone} is at {actual} on {written}, not {}; write "{written}{actual} {zone}" or drop the offset"#,
                    show(given)
                ));
            }
            actual
        }
        (AmbiguousOffset::Gap { before, after }, _) => {
            // The transition is the first one after the local time read with the later offset.
            let probe = after
                .to_timestamp(civil)
                .map_err(|_| too_late(written, zone))?;
            let at = tz
                .following(probe)
                .next()
                .map(|t| t.timestamp())
                .ok_or("no transition after a gap")?;
            let (from, to) = (before.to_datetime(at), after.to_datetime(at));
            // Show the dates too when the jump crosses midnight.
            let local = |t: DateTime| match from.date() == to.date() {
                true => format!("{:02}:{:02}", t.hour(), t.minute()),
                false => format!("{} {:02}:{:02}", t.date(), t.hour(), t.minute()),
            };
            return Err(format!(
                "{written} doesn't exist in {zone}: the clocks jump from {} to {} ({} to {})",
                local(from),
                local(to),
                show(before.seconds()),
                show(after.seconds())
            ));
        }
        (AmbiguousOffset::Fold { before, after }, None) => {
            let (first, second) = (show(before.seconds()), show(after.seconds()));
            return Err(format!(
                r#"{written} happens twice in {zone}; write "{written}{first} {zone}" (the first) or "{written}{second} {zone}" (the second)"#
            ));
        }
        (AmbiguousOffset::Fold { before, after }, Some(given)) => {
            match [before, after].into_iter().find(|o| o.seconds() == given) {
                Some(offset) => offset,
                None => {
                    return Err(format!(
                        "{written} in {zone} is {} or {}, not {}",
                        show(before.seconds()),
                        show(after.seconds()),
                        show(given)
                    ));
                }
            }
        }
    };
    let instant = chosen
        .to_timestamp(civil)
        .map_err(|_| too_late(written, zone))?;
    let nanos = instant.subsec_nanosecond();
    Ok((
        instant.as_second() - i64::from(nanos < 0),
        nanos.rem_euclid(1_000_000_000) as u32,
        chosen.seconds(),
    ))
}

/// Finds `zone` in the bundled database, which is the only one `ts!` uses, so the
/// result doesn't depend on the build machine.
fn lookup(zone: &str) -> Result<TimeZone, String> {
    let db = TimeZoneDatabase::bundled();
    if let Ok(tz) = db.get(zone) {
        return match tz.iana_name() {
            Some(name) if name != zone => Err(format!(r#"write the zone as "{name}""#)),
            _ => Ok(tz),
        };
    }
    let abbreviation =
        (2..=5).contains(&zone.len()) && zone.bytes().all(|b| b.is_ascii_uppercase());
    if abbreviation {
        return Err(format!(
            r#"unknown time zone "{zone}"; abbreviations aren't zone names, use an offset like "-08:00" or a name like "America/Los_Angeles""#
        ));
    }
    let closest = db
        .available()
        .map(|name| name.to_string())
        .map(|name| (levenshtein(zone, &name), name))
        .min();
    Err(match closest {
        // At most 3 edits, and at most one per three characters of the name.
        Some((distance, name)) if distance <= 3 && 3 * distance <= zone.len() => {
            format!(r#"unknown time zone "{zone}"; did you mean "{name}"?"#)
        }
        _ => format!(r#"unknown time zone "{zone}""#),
    })
}

/// jiff's timestamps, and so named zones, end about a day before year 9999 does;
/// a fixed offset reaches the end of 9999.
fn too_late(written: &str, zone: &str) -> String {
    format!(
        r#"{written} is too late for a named zone, which ends at 9999-12-30 22:00 UTC; write an offset instead of "{zone}""#
    )
}

/// `±HH:MM`, or `±HH:MM:SS` when the offset has seconds.
fn show(seconds: i32) -> String {
    let sign = if seconds < 0 { '-' } else { '+' };
    let magnitude = seconds.unsigned_abs();
    let hm = format!("{sign}{:02}:{:02}", magnitude / 3600, magnitude / 60 % 60);
    match magnitude % 60 {
        0 => hm,
        s => format!("{hm}:{s:02}"),
    }
}

fn levenshtein(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            let above = row[j + 1];
            row[j + 1] = (above + 1)
                .min(row[j] + 1)
                .min(diagonal + usize::from(ca != cb));
            diagonal = above;
        }
    }
    row[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zoned(s: &str) -> (i64, u32, i32, String) {
        match crate::ts::parse_ts(s) {
            Ok(crate::ts::Ts::Zoned {
                secs,
                nanos,
                offset,
                zone,
            }) => (secs, nanos, offset, zone),
            other => panic!("{s:?}: {other:?}"),
        }
    }

    #[test]
    fn zone_values() {
        assert_eq!(
            zoned("2026-10-30 14:00 Europe/Stockholm"),
            (1_793_365_200, 0, 3600, "Europe/Stockholm".into())
        );
        assert_eq!(
            zoned("2026-10-25 02:30+01:00 Europe/Stockholm"),
            (1_792_891_800, 0, 3600, "Europe/Stockholm".into())
        );
        assert_eq!(
            zoned("2026-10-25 02:30+02:00 Europe/Stockholm"),
            (1_792_888_200, 0, 7200, "Europe/Stockholm".into())
        );
        assert_eq!(
            zoned("2026-10-30T14:00 UTC"),
            (1_793_368_800, 0, 0, "UTC".into())
        );
        assert_eq!(
            zoned("2026-10-30 14:00:05.5 Asia/Calcutta"),
            (
                1_793_368_800 - 19_800 + 5,
                500_000_000,
                19_800,
                "Asia/Calcutta".into()
            )
        );
    }

    #[test]
    fn zone_errors() {
        let cases = [
            (
                "2026-10-30 14:00 Europe/Stokholm",
                r#"unknown time zone "Europe/Stokholm"; did you mean "Europe/Stockholm"?"#,
            ),
            (
                "2026-10-30 14:00 Nowhere/Atall",
                r#"unknown time zone "Nowhere/Atall""#,
            ),
            // No far-fetched suggestions for short names.
            ("2026-10-30 14:00 /", r#"unknown time zone "/""#),
            ("2026-10-30 14:00 ab", r#"unknown time zone "ab""#),
            (
                "2026-10-30 14:00 PST",
                r#"unknown time zone "PST"; abbreviations aren't zone names, use an offset like "-08:00" or a name like "America/Los_Angeles""#,
            ),
            (
                "2026-10-30 14:00 europe/stockholm",
                r#"write the zone as "Europe/Stockholm""#,
            ),
            (
                "2026-03-29 02:30 Europe/Stockholm",
                "2026-03-29 02:30 doesn't exist in Europe/Stockholm: the clocks jump from 02:00 to 03:00 (+01:00 to +02:00)",
            ),
            // New York's spring and autumn changes are at different UTC times.
            (
                "2026-03-08 02:30 America/New_York",
                "2026-03-08 02:30 doesn't exist in America/New_York: the clocks jump from 02:00 to 03:00 (-05:00 to -04:00)",
            ),
            (
                "2026-10-04 02:15 Australia/Lord_Howe",
                "2026-10-04 02:15 doesn't exist in Australia/Lord_Howe: the clocks jump from 02:00 to 02:30 (+10:30 to +11:00)",
            ),
            // A whole skipped day shows the dates.
            (
                "2011-12-30 12:00 Pacific/Apia",
                "2011-12-30 12:00 doesn't exist in Pacific/Apia: the clocks jump from 2011-12-30 00:00 to 2011-12-31 00:00 (-10:00 to +14:00)",
            ),
            // jiff's timestamps end at 9999-12-30T22:00Z.
            (
                "9999-12-31 12:00 Europe/Paris",
                r#"9999-12-31 12:00 is too late for a named zone, which ends at 9999-12-30 22:00 UTC; write an offset instead of "Europe/Paris""#,
            ),
            (
                "9999-12-31T00:00 UTC",
                r#"9999-12-31T00:00 is too late for a named zone, which ends at 9999-12-30 22:00 UTC; write an offset instead of "UTC""#,
            ),
            (
                "2026-03-29 02:30+01:00 Europe/Stockholm",
                "2026-03-29 02:30 doesn't exist in Europe/Stockholm: the clocks jump from 02:00 to 03:00 (+01:00 to +02:00)",
            ),
            (
                "2026-10-25 02:30 Europe/Stockholm",
                r#"2026-10-25 02:30 happens twice in Europe/Stockholm; write "2026-10-25 02:30+02:00 Europe/Stockholm" (the first) or "2026-10-25 02:30+01:00 Europe/Stockholm" (the second)"#,
            ),
            (
                "2026-10-25T02:30:15.5 Europe/Stockholm",
                r#"2026-10-25T02:30:15.5 happens twice in Europe/Stockholm; write "2026-10-25T02:30:15.5+02:00 Europe/Stockholm" (the first) or "2026-10-25T02:30:15.5+01:00 Europe/Stockholm" (the second)"#,
            ),
            (
                "2026-10-25 02:30+03:00 Europe/Stockholm",
                "2026-10-25 02:30 in Europe/Stockholm is +02:00 or +01:00, not +03:00",
            ),
            (
                "2026-11-01 14:00+02:00 Europe/Stockholm",
                r#"Europe/Stockholm is at +01:00 on 2026-11-01 14:00, not +02:00; write "2026-11-01 14:00+01:00 Europe/Stockholm" or drop the offset"#,
            ),
            (
                "2026-04-05 01:45 Australia/Lord_Howe",
                r#"2026-04-05 01:45 happens twice in Australia/Lord_Howe; write "2026-04-05 01:45+11:00 Australia/Lord_Howe" (the first) or "2026-04-05 01:45+10:30 Australia/Lord_Howe" (the second)"#,
            ),
        ];
        for (input, message) in cases {
            assert_eq!(
                crate::ts::parse_ts(input).expect_err(input),
                message,
                "input {input:?}"
            );
        }
    }

    #[test]
    fn tzdb_version_text() {
        assert_eq!(tzdb_version(), jiff_tzdb::VERSION.unwrap_or("unknown"));
        assert!(!tzdb_version().is_empty());
    }

    /// The resolver is built on jiff, so the oracle is chrono-tz, an independent
    /// reading of the tz database.
    #[test]
    fn zone_oracle() {
        use chrono::{LocalResult, NaiveDate, Offset as _, TimeZone as _};
        use jiff::{Span, tz::TimeZone};
        const ZONES: [&str; 8] = [
            "Europe/Stockholm",
            "Europe/Dublin",
            "America/New_York",
            "America/St_Johns",
            "Australia/Lord_Howe",
            "Pacific/Apia",
            "Asia/Kolkata",
            "Africa/Casablanca",
        ];
        let db = jiff::tz::TimeZoneDatabase::bundled();
        let start = jiff::civil::date(1990, 1, 1)
            .at(0, 0, 0, 0)
            .to_zoned(TimeZone::UTC)
            .unwrap()
            .timestamp();
        let end = jiff::civil::date(2025, 1, 1)
            .at(0, 0, 0, 0)
            .to_zoned(TimeZone::UTC)
            .unwrap()
            .timestamp();
        let (mut checked, mut transitions, mut skipped) = (0, 0, 0);
        for name in ZONES {
            let tz = db.get(name).unwrap();
            let theirs: chrono_tz::Tz = name.parse().unwrap();
            for transition in tz.following(start).take_while(|t| t.timestamp() < end) {
                // The two crates bundle their own copies of the tz database. Where
                // the copies disagree about a transition, the data differs, not the
                // resolver, so skip it; `skipped` below bounds how often.
                let offset_at = |second: i64| {
                    let utc = chrono::DateTime::from_timestamp(second, 0)
                        .unwrap()
                        .naive_utc();
                    theirs
                        .offset_from_utc_datetime(&utc)
                        .fix()
                        .local_minus_utc()
                };
                let at = transition.timestamp().as_second();
                let previous = tz.to_offset(transition.timestamp() - Span::new().seconds(1));
                if offset_at(at - 1) != previous.seconds()
                    || offset_at(at) != transition.offset().seconds()
                {
                    skipped += 1;
                    continue;
                }
                transitions += 1;
                let local = transition.timestamp().to_zoned(tz.clone()).datetime();
                for step in -6..=6 {
                    let dt = local.checked_add(Span::new().minutes(15 * step)).unwrap();
                    let (y, mo, d) = (dt.year(), dt.month() as u8, dt.day() as u8);
                    let (h, mi) = (dt.hour() as u8, dt.minute() as u8);
                    let written = format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02}");
                    let resolve_at =
                        |offset| resolve(&written, (y, mo, d), (h, mi, 0, 0), offset, name);
                    let naive = NaiveDate::from_ymd_opt(y.into(), mo.into(), d.into())
                        .unwrap()
                        .and_hms_opt(h.into(), mi.into(), 0)
                        .unwrap();
                    let instant = |t: chrono::DateTime<chrono_tz::Tz>| {
                        (t.timestamp(), 0, t.offset().fix().local_minus_utc())
                    };
                    match theirs.from_local_datetime(&naive) {
                        LocalResult::Single(t) => {
                            assert_eq!(resolve_at(None), Ok(instant(t)), "{written} {name}");
                        }
                        LocalResult::Ambiguous(a, b) => {
                            let message = resolve_at(None).expect_err(&written);
                            assert!(message.contains("happens twice"), "{message}");
                            for t in [a, b] {
                                let (_, _, offset) = instant(t);
                                assert_eq!(
                                    resolve_at(Some(offset)),
                                    Ok(instant(t)),
                                    "{written} {name}"
                                );
                            }
                        }
                        LocalResult::None => {
                            let message = resolve_at(None).expect_err(&written);
                            assert!(message.contains("doesn't exist"), "{message}");
                        }
                    }
                    checked += 1;
                }
            }
        }
        assert!(checked > 5_000, "only {checked} cases");
        assert!(
            skipped * 20 < transitions,
            "the databases disagree on {skipped} of {} transitions",
            transitions + skipped
        );
    }
}
