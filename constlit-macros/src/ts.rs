//! `ts!`: a date, a time and a zone (`Z`, an offset, or a zone name).

use crate::datetime::{all_digits, days_from_civil, parse_date, parse_time};

/// A parsed `ts!` literal: an instant as Unix seconds and nanoseconds (floored),
/// and the UTC offset in seconds.
#[derive(Debug, PartialEq)]
pub enum Ts {
    Fixed {
        secs: i64,
        nanos: u32,
        offset: i32,
    },
    // Only `ts!` with the `tz` feature makes these.
    #[cfg_attr(not(feature = "tz"), allow(dead_code))]
    Zoned {
        secs: i64,
        nanos: u32,
        offset: i32,
        zone: String,
    },
}

const EXAMPLE: &str = r#"write a date, a time and a zone, like "2026-10-30T14:00Z""#;

/// Parses `DATE ("T" | " ") TIME ("Z" | OFFSET) [" " ZONE]` or `DATE ("T" | " ") TIME " " ZONE`.
pub fn parse_ts(s: &str) -> Result<Ts, String> {
    if s.is_empty() {
        return Err(format!("empty timestamp; {EXAMPLE}"));
    }
    if s.starts_with(char::is_whitespace) || s.ends_with(char::is_whitespace) {
        return Err("remove the spaces around the literal".into());
    }
    let Some(sep_at) = s.find(['T', 't', ' ', '_']) else {
        parse_date(s)?;
        return Err(format!("missing time; {EXAMPLE}"));
    };
    let date = parse_date(&s[..sep_at])?;
    let rest = &s[sep_at + 1..];
    match &s[sep_at..=sep_at] {
        "t" => {
            return Err(format!(
                r#"write "T" in uppercase: "{}""#,
                replace_at(s, sep_at, "T")
            ));
        }
        "_" => return Err(r#"separate the date and time with "T" or one space"#.into()),
        " " if rest.starts_with(' ') => {
            return Err(r#"separate the date and time with "T" or one space"#.into());
        }
        _ => {}
    }
    let time_len = rest
        .find(['Z', 'z', '+', '-', '[', ' '])
        .unwrap_or(rest.len());
    let time_text = &rest[..time_len];
    let time = parse_time(time_text)?;
    let tail = &rest[time_len..];
    let tail_at = s.len() - tail.len();
    let (offset, after) = match tail.chars().next() {
        None => {
            return Err(
                r#"missing time zone; add "Z" for UTC, an offset like "+01:00", or a zone name like "Europe/Stockholm""#
                    .into(),
            );
        }
        Some('z') => {
            return Err(format!(
                r#"write "Z" in uppercase: "{}""#,
                replace_at(s, tail_at, "Z")
            ));
        }
        Some('Z') => (Some(0), &tail[1..]),
        Some('+' | '-') => {
            let end = tail.find([' ', '[']).unwrap_or(tail.len());
            (Some(parse_offset(&tail[..end])?), &tail[end..])
        }
        Some(' ') if tail[1..].starts_with(['+', '-']) => {
            let offset = &tail[1..];
            let offset = &offset[..offset.find([' ', '[']).unwrap_or(offset.len())];
            return Err(format!(
                r#"remove the space before the offset: "{time_text}{offset}""#
            ));
        }
        _ => (None, tail),
    };
    if let Some(name) = after.strip_prefix('[') {
        let name = name.strip_suffix(']').unwrap_or(name);
        return Err(format!(
            r#"write the zone after a space: "{} {name}""#,
            &s[..s.len() - after.len()]
        ));
    }
    if tail.starts_with('Z') && !after.is_empty() && !after.starts_with(' ') {
        return Err(format!(r#"unexpected text after "Z": "{after}""#));
    }
    let local = local_seconds(date, time);
    let Some(zone) = after.strip_prefix(' ') else {
        let offset = offset.expect("a missing zone was reported above");
        return Ok(Ts::Fixed {
            secs: local - i64::from(offset),
            nanos: time.3,
            offset,
        });
    };
    if tail.starts_with('Z') {
        return Err(r#"write an offset like "+01:00" before a zone name, or remove "Z""#.into());
    }
    if zone == "Z" {
        return Err(format!(
            r#"remove the space before "Z": "{}Z""#,
            &s[..s.len() - after.len()]
        ));
    }
    let written = &s[..sep_at + 1 + time_len];
    zoned(written, date, time, offset, zone)
}

#[cfg(not(feature = "tz"))]
fn zoned(
    _written: &str,
    _date: (i16, u8, u8),
    _time: (u8, u8, u8, u32),
    _offset: Option<i32>,
    _zone: &str,
) -> Result<Ts, String> {
    Err(r#"named time zones need constlit's "tz" feature; enable it, or write an offset like "+01:00""#.into())
}

#[cfg(feature = "tz")]
fn zoned(
    written: &str,
    date: (i16, u8, u8),
    time: (u8, u8, u8, u32),
    offset: Option<i32>,
    zone: &str,
) -> Result<Ts, String> {
    let (secs, nanos, offset) = crate::zone::resolve(written, date, time, offset, zone)?;
    Ok(Ts::Zoned {
        secs,
        nanos,
        offset,
        zone: zone.to_string(),
    })
}

/// `s` with the one-byte character at `at` replaced.
fn replace_at(s: &str, at: usize, with: &str) -> String {
    format!("{}{with}{}", &s[..at], &s[at + 1..])
}

/// Seconds from 1970-01-01T00:00 to the local date and time, ignoring the fraction.
fn local_seconds(
    (year, month, day): (i16, u8, u8),
    (hour, minute, second, _): (u8, u8, u8, u32),
) -> i64 {
    days_from_civil(year, month, day) * 86_400
        + i64::from(hour) * 3600
        + i64::from(minute) * 60
        + i64::from(second)
}

/// Parses `±HH:MM`, `±HHMM` or `±HH:MM:SS` into seconds east of UTC.
fn parse_offset(text: &str) -> Result<i32, String> {
    let sign = &text[..1];
    let body = &text[1..];
    let parts: Vec<&str> = if body.contains(':') {
        body.split(':').collect()
    } else if body.len() == 4 && all_digits(body) {
        vec![&body[..2], &body[2..]]
    } else {
        vec![body]
    };
    if parts.is_empty() || parts.len() > 3 || !parts.iter().all(|p| all_digits(p) && p.len() <= 2) {
        return Err(r#"write the offset as ±HH:MM, like "+01:00""#.into());
    }
    let values: Vec<i32> = parts.iter().map(|p| p.parse().expect("digits")).collect();
    if parts.len() == 1 || parts.iter().any(|p| p.len() != 2) {
        let minutes = values.get(1).copied().unwrap_or(0);
        return Err(format!(
            r#"write the offset as ±HH:MM: "{sign}{:02}:{minutes:02}""#,
            values[0]
        ));
    }
    let (hours, minutes, seconds) = (values[0], values[1], values.get(2).copied().unwrap_or(0));
    if hours > 23 || minutes > 59 || seconds > 59 {
        return Err(format!(
            "offset {text} is out of range (-23:59:59 to +23:59:59)"
        ));
    }
    let magnitude = hours * 3600 + minutes * 60 + seconds;
    match (sign, magnitude) {
        ("-", 0) => {
            Err(r#""-00:00" means an unknown offset (RFC 3339); write "Z" or "+00:00""#.into())
        }
        ("-", m) => Ok(-m),
        (_, m) => Ok(m),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixed(s: &str) -> (i64, u32, i32) {
        match parse_ts(s) {
            Ok(Ts::Fixed {
                secs,
                nanos,
                offset,
            }) => (secs, nanos, offset),
            other => panic!("{s:?}: {other:?}"),
        }
    }

    #[test]
    fn ts_values() {
        assert_eq!(fixed("1970-01-01T00:00Z"), (0, 0, 0));
        assert_eq!(fixed("2026-10-30T14:00:00Z"), (1_793_368_800, 0, 0));
        assert_eq!(fixed("2026-10-30 15:00+01:00"), (1_793_368_800, 0, 3600));
        assert_eq!(
            fixed("2026-10-30T14:00-0530"),
            (1_793_368_800 + 19_800, 0, -19_800)
        );
        assert_eq!(
            fixed("2026-10-30T14:00+05:30:15"),
            (1_793_368_800 - 19_815, 0, 19_815)
        );
        assert_eq!(fixed("1969-12-31T23:59:59.5Z"), (-1, 500_000_000, 0));
    }

    #[test]
    fn ts_errors() {
        let cases = [
            (
                "",
                r#"empty timestamp; write a date, a time and a zone, like "2026-10-30T14:00Z""#,
            ),
            (
                "2026-10-30",
                r#"missing time; write a date, a time and a zone, like "2026-10-30T14:00Z""#,
            ),
            (
                "2026-10-30T14:00",
                r#"missing time zone; add "Z" for UTC, an offset like "+01:00", or a zone name like "Europe/Stockholm""#,
            ),
            (
                "2026-10-30 14:00",
                r#"missing time zone; add "Z" for UTC, an offset like "+01:00", or a zone name like "Europe/Stockholm""#,
            ),
            (
                "2026-10-30t14:00Z",
                r#"write "T" in uppercase: "2026-10-30T14:00Z""#,
            ),
            (
                "2026-10-30T14:00z",
                r#"write "Z" in uppercase: "2026-10-30T14:00Z""#,
            ),
            (
                "2026-10-30  14:00Z",
                r#"separate the date and time with "T" or one space"#,
            ),
            (
                "2026-10-30_14:00Z",
                r#"separate the date and time with "T" or one space"#,
            ),
            (
                "2026-10-30T14:00+02",
                r#"write the offset as ±HH:MM: "+02:00""#,
            ),
            (
                "2026-10-30T14:00+2:00",
                r#"write the offset as ±HH:MM: "+02:00""#,
            ),
            (
                "2026-10-30T14:00+24:00",
                "offset +24:00 is out of range (-23:59:59 to +23:59:59)",
            ),
            (
                "2026-10-30T14:00+01:60",
                "offset +01:60 is out of range (-23:59:59 to +23:59:59)",
            ),
            (
                "2026-10-30T14:00-00:00",
                r#""-00:00" means an unknown offset (RFC 3339); write "Z" or "+00:00""#,
            ),
            (
                "2026-10-30T14:00 +01:00",
                r#"remove the space before the offset: "14:00+01:00""#,
            ),
            (
                "2026-10-30T14:00Z Europe/Stockholm",
                r#"write an offset like "+01:00" before a zone name, or remove "Z""#,
            ),
            (
                "2026-10-30T14:00+01:00[Europe/Stockholm]",
                r#"write the zone after a space: "2026-10-30T14:00+01:00 Europe/Stockholm""#,
            ),
            (
                "2026-10-30T14:00Zfoo",
                r#"unexpected text after "Z": "foo""#,
            ),
            ("2026-10-30T14:00ZZ", r#"unexpected text after "Z": "Z""#),
            (
                "2026-10-30T14:00Z+01:00",
                r#"unexpected text after "Z": "+01:00""#,
            ),
            (
                "2026-10-30T14:00+0é0",
                r#"write the offset as ±HH:MM, like "+01:00""#,
            ),
            (
                "2026-10-30T14:00-1€",
                r#"write the offset as ±HH:MM, like "+01:00""#,
            ),
            (
                "2026-10-30 14:00 Z",
                r#"remove the space before "Z": "2026-10-30 14:00Z""#,
            ),
            // The date and time parts report their own errors.
            (
                "2026-02-30T14:00Z",
                "day 30 is out of range: February 2026 has 28 days",
            ),
            (
                "2026-10-30T24:00Z",
                r#"hour 24 is out of range (00-23); midnight is "00:00""#,
            ),
        ];
        for (input, message) in cases {
            assert_eq!(
                parse_ts(input).expect_err(input),
                message,
                "input {input:?}"
            );
        }
    }

    #[cfg(not(feature = "tz"))]
    #[test]
    fn zone_needs_tz() {
        assert_eq!(
            parse_ts("2026-10-30 14:00 Europe/Stockholm").expect_err("zone"),
            r#"named time zones need constlit's "tz" feature; enable it, or write an offset like "+01:00""#
        );
    }

    fn splitmix(state: &mut u64) -> u64 {
        *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = *state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    #[test]
    fn ts_oracle() {
        use jiff::civil::DateTime;
        use jiff::tz::{Offset, TimeZone};
        let mut state = 13u64;
        let mut r = |n: u64| splitmix(&mut state) % n;
        for _ in 0..20_000 {
            let year = 1 + r(9999) as i16;
            let month = 1 + r(12) as i8;
            let day = 1 + r(crate::datetime::days_in_month(year, month as u8) as u64) as i8;
            let (hour, minute, second) = (r(24) as i8, r(60) as i8, r(60) as i8);
            let digits = r(10) as usize;
            let nanos = if digits == 0 {
                0
            } else {
                let unit = 10u64.pow(9 - digits as u32);
                (r(1_000_000_000 / unit) * unit) as i32
            };
            let offset = r(2 * 86_340 + 1) as i32 / 60 * 60 - 86_340;
            let fraction = if digits == 0 {
                String::new()
            } else {
                format!(".{:09}", nanos)[..=digits].to_string()
            };
            let sign = if offset < 0 { '-' } else { '+' };
            let text = format!(
                "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}{fraction}{sign}{:02}:{:02}",
                offset.abs() / 3600,
                offset.abs() / 60 % 60
            );
            if text.ends_with("-00:00") {
                continue;
            }
            let instant = DateTime::new(year, month, day, hour, minute, second, nanos)
                .unwrap()
                .to_zoned(TimeZone::fixed(Offset::from_seconds(offset).unwrap()))
                .unwrap()
                .timestamp();
            let expected = (
                instant.as_second() - i64::from(instant.subsec_nanosecond() < 0),
                instant.subsec_nanosecond().rem_euclid(1_000_000_000) as u32,
                offset,
            );
            assert_eq!(fixed(&text), expected, "{text}");
        }
    }
}
