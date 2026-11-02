//! The date and time grammar shared by `date!`, `time!` and `ts!`.

pub const MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

const ISO: &str = r#"write the date as YYYY-MM-DD, like "2026-10-30""#;

fn is_leap(year: i16) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

/// Days in `month` (1-12) of `year`.
pub fn days_in_month(year: i16, month: u8) -> u8 {
    match month {
        2 if is_leap(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Days from 1970-01-01 to the given date, by Howard Hinnant's `days_from_civil`.
pub fn days_from_civil(year: i16, month: u8, day: u8) -> i64 {
    let y = i64::from(year) - i64::from(month <= 2);
    let era = y.div_euclid(400);
    let year_of_era = y.rem_euclid(400);
    let shifted_month = (i64::from(month) + 9) % 12;
    let day_of_year = (153 * shifted_month + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

pub(crate) fn all_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

/// `"3"` → `"03"`.
fn pad2(s: &str) -> String {
    format!("{s:0>2}")
}

/// Parses an ISO 8601 calendar date, `YYYY-MM-DD` with a year in 0001-9999.
pub fn parse_date(s: &str) -> Result<(i16, u8, u8), String> {
    if s.is_empty() {
        return Err(r#"empty date; write YYYY-MM-DD, like "2026-10-30""#.into());
    }
    if s.starts_with(char::is_whitespace) || s.ends_with(char::is_whitespace) {
        return Err("remove the spaces around the literal".into());
    }
    if s.contains(['T', ' ', ':']) {
        return Err("date! takes only a date; use ts! for a date with a time and zone".into());
    }
    if s.contains(['W', 'w']) {
        return Err("write the date as YYYY-MM-DD; week and ordinal dates aren't supported".into());
    }
    let Some(sep) = s.chars().find(|c| !c.is_ascii_alphanumeric()) else {
        return Err(if s.len() == 8 && all_digits(s) {
            format!(r#"add the dashes: "{}-{}-{}""#, &s[..4], &s[4..6], &s[6..])
        } else {
            ISO.into()
        });
    };
    let parts: Vec<&str> = s.split(sep).collect();
    let year_last = parts.len() == 3 && parts[2].len() == 4 && parts[0].len() <= 2;
    if year_last && parts.iter().all(|p| all_digits(p)) {
        return Err("write the date year first, as YYYY-MM-DD (ISO 8601)".into());
    }
    if sep != '-' {
        if parts.len() == 3 && parts[0].len() == 4 && parts.iter().all(|p| all_digits(p)) {
            return Err(format!(
                r#"use "-" between year, month and day: "{}-{}-{}""#,
                parts[0],
                pad2(parts[1]),
                pad2(parts[2])
            ));
        }
        return Err(ISO.into());
    }
    let (year, month, day) = match parts[..] {
        [year, month, day] => (year, month, day),
        [year, ordinal] if ordinal.len() == 3 && all_digits(ordinal) && all_digits(year) => {
            return Err(
                "write the date as YYYY-MM-DD; week and ordinal dates aren't supported".into(),
            );
        }
        [year, month] if all_digits(year) && all_digits(month) => {
            return Err(format!(
                r#"missing day; write YYYY-MM-DD, like "{year}-{}-01""#,
                pad2(month)
            ));
        }
        _ => return Err(ISO.into()),
    };
    for (name, part) in [("year", year), ("month", month), ("day", day)] {
        if !all_digits(part) {
            return Err(format!("{name} `{part}` is not a number"));
        }
    }
    if year.len() < 4 {
        return Err(format!(
            r#"write the year with all four digits; "{year}" could be any century"#
        ));
    }
    let y: i16 = match year.parse() {
        Ok(y @ 1..=9999) if year.len() == 4 => y,
        _ => return Err(format!("year {year} is out of range (0001-9999)")),
    };
    let m: u8 = match month.parse() {
        Ok(m @ 1..=12) => m,
        _ => return Err(format!("month {month} is out of range (01-12)")),
    };
    let length = days_in_month(y, m);
    let d: u8 = match day.parse() {
        Ok(d) if (1..=length).contains(&d) => d,
        _ => {
            return Err(format!(
                "day {day} is out of range: {} {y} has {length} days",
                MONTH_NAMES[m as usize - 1]
            ));
        }
    };
    if month.len() != 2 || day.len() != 2 {
        return Err(format!(
            r#"write the month and day with two digits: "{year}-{m:02}-{d:02}""#
        ));
    }
    Ok((y, m, d))
}

const TIME_NAMES: [&str; 3] = ["hour", "minute", "second"];

/// Parses a 24-hour time of day: `HH:MM`, `HH:MM:SS` or `HH:MM:SS.F` with 1-9
/// fraction digits. Returns hour, minute, second and nanosecond.
pub fn parse_time(s: &str) -> Result<(u8, u8, u8, u32), String> {
    if s.is_empty() {
        return Err(r#"empty time; write HH:MM or HH:MM:SS, like "14:30""#.into());
    }
    if s.starts_with(char::is_whitespace) || s.ends_with(char::is_whitespace) {
        return Err("remove the spaces around the literal".into());
    }
    if let Some(rest) = s.strip_prefix('T') {
        parse_time(rest)?;
        return Err(format!(r#"remove the leading "T": "{rest}""#));
    }
    if let Some(i) = s.find(['Z', '+', '-']) {
        return Err(format!(
            r#"time! takes a time of day without a zone; remove "{}", or use ts!"#,
            &s[i..]
        ));
    }
    if let Some(fixed) = twelve_hour(s) {
        return Err(match fixed {
            Some(fixed) => format!(r#"use the 24-hour clock: "{fixed}""#),
            None => r#"use the 24-hour clock HH:MM, like "14:30""#.into(),
        });
    }
    if s.contains(['h', 'H']) {
        return Err(r#"use the 24-hour clock HH:MM, like "14:30""#.into());
    }
    if s.contains(',') {
        return Err(format!(
            r#"use "." before the fraction: "{}""#,
            s.replace(',', ".")
        ));
    }
    let (main, fraction) = match s.split_once('.') {
        Some((main, fraction)) => (main, Some(fraction)),
        None => (s, None),
    };
    let nanosecond = match fraction {
        None => 0,
        Some("") => return Err(r#"missing digits after ".""#.into()),
        Some(f) if !all_digits(f) => return Err(format!("fraction `{f}` is not a number")),
        Some(f) if f.len() > 9 => return Err("at most 9 fractional digits (nanoseconds)".into()),
        Some(f) => format!("{f:0<9}").parse().expect("nine digits"),
    };
    if !main.contains(':') {
        return Err(match main.len() {
            1 | 2 if all_digits(main) => {
                format!(r#"missing minutes; write HH:MM, like "{main:0>2}:00""#)
            }
            4 if all_digits(main) => format!(r#"add the colons: "{}:{}""#, &main[..2], &main[2..]),
            6 if all_digits(main) => format!(
                r#"add the colons: "{}:{}:{}""#,
                &main[..2],
                &main[2..4],
                &main[4..]
            ),
            _ => r#"write the time as HH:MM or HH:MM:SS, like "14:30""#.into(),
        });
    }
    let parts: Vec<&str> = main.split(':').collect();
    if parts.len() > 3 {
        return Err("too many parts; write HH:MM:SS".into());
    }
    let mut values = [0u8; 3];
    for ((name, part), value) in TIME_NAMES.iter().zip(&parts).zip(&mut values) {
        if !all_digits(part) {
            return Err(format!("{name} `{part}` is not a number"));
        }
        *value = part.parse().unwrap_or(u8::MAX);
    }
    if fraction.is_some() && parts.len() == 2 {
        return Err("a fraction needs seconds: write HH:MM:SS.F".into());
    }
    let [hour, minute, second] = values;
    match (hour, minute, second) {
        (24, ..) => {
            return Err(r#"hour 24 is out of range (00-23); midnight is "00:00""#.into());
        }
        (24.., ..) => return Err(format!("hour {} is out of range (00-23)", parts[0])),
        (_, 60.., _) => return Err(format!("minute {} is out of range (00-59)", parts[1])),
        (.., 60) => {
            return Err("second 60 is out of range (00-59); leap seconds aren't supported".into());
        }
        (.., 61..) => return Err(format!("second {} is out of range (00-59)", parts[2])),
        _ => {}
    }
    if parts.iter().any(|p| p.len() != 2) {
        let mut fixed = values[..parts.len()]
            .iter()
            .map(|v| format!("{v:02}"))
            .collect::<Vec<_>>()
            .join(":");
        if let Some(f) = fraction {
            fixed = format!("{fixed}.{f}");
        }
        return Err(format!(r#"write each part with two digits: "{fixed}""#));
    }
    Ok((hour, minute, second, nanosecond))
}

/// For a time ending in AM or PM: `Some(Some(fixed))` with the 24-hour spelling
/// when the rest is a valid 12-hour time, `Some(None)` otherwise.
fn twelve_hour(s: &str) -> Option<Option<String>> {
    let lower = s.to_ascii_lowercase();
    let (base, pm) = match (lower.strip_suffix("am"), lower.strip_suffix("pm")) {
        (Some(base), _) => (base, false),
        (_, Some(base)) => (base, true),
        _ => return None,
    };
    let base = base.strip_suffix(' ').unwrap_or(base);
    let (hour, rest) = base.split_once(':')?;
    let fixed = match hour.parse::<u8>() {
        Ok(h @ 1..=12) if all_digits(hour) && hour.len() <= 2 => {
            let h = h % 12 + if pm { 12 } else { 0 };
            Some(format!("{h:02}:{rest}")).filter(|fixed| parse_time(fixed).is_ok())
        }
        _ => None,
    };
    Some(fixed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date_err(s: &str) -> String {
        parse_date(s).expect_err(s)
    }

    #[test]
    fn date_values() {
        assert_eq!(parse_date("2026-10-30"), Ok((2026, 10, 30)));
        assert_eq!(parse_date("0001-01-01"), Ok((1, 1, 1)));
        assert_eq!(parse_date("9999-12-31"), Ok((9999, 12, 31)));
        assert_eq!(parse_date("2024-02-29"), Ok((2024, 2, 29)));
        assert_eq!(parse_date("2000-02-29"), Ok((2000, 2, 29)));
    }

    #[test]
    fn date_errors() {
        let cases = [
            ("", r#"empty date; write YYYY-MM-DD, like "2026-10-30""#),
            (
                "2026-2-3",
                r#"write the month and day with two digits: "2026-02-03""#,
            ),
            (
                "26-10-30",
                r#"write the year with all four digits; "26" could be any century"#,
            ),
            (
                "2026/10/30",
                r#"use "-" between year, month and day: "2026-10-30""#,
            ),
            (
                "2026.10.30",
                r#"use "-" between year, month and day: "2026-10-30""#,
            ),
            (
                "30-10-2026",
                "write the date year first, as YYYY-MM-DD (ISO 8601)",
            ),
            (
                "10/30/2026",
                "write the date year first, as YYYY-MM-DD (ISO 8601)",
            ),
            (
                "30.10.2026",
                "write the date year first, as YYYY-MM-DD (ISO 8601)",
            ),
            ("20261030", r#"add the dashes: "2026-10-30""#),
            (
                "2026-W44-5",
                "write the date as YYYY-MM-DD; week and ordinal dates aren't supported",
            ),
            (
                "2026-303",
                "write the date as YYYY-MM-DD; week and ordinal dates aren't supported",
            ),
            (
                "2026-10",
                r#"missing day; write YYYY-MM-DD, like "2026-10-01""#,
            ),
            ("2026-1x-30", "month `1x` is not a number"),
            ("20x6-10-30", "year `20x6` is not a number"),
            ("2026-10-3x", "day `3x` is not a number"),
            ("0000-01-01", "year 0000 is out of range (0001-9999)"),
            ("12026-01-01", "year 12026 is out of range (0001-9999)"),
            ("2026-13-01", "month 13 is out of range (01-12)"),
            ("2026-00-10", "month 00 is out of range (01-12)"),
            (
                "2026-02-30",
                "day 30 is out of range: February 2026 has 28 days",
            ),
            (
                "2026-10-00",
                "day 00 is out of range: October 2026 has 31 days",
            ),
            (
                "1900-02-29",
                "day 29 is out of range: February 1900 has 28 days",
            ),
            (
                "2025-02-29",
                "day 29 is out of range: February 2025 has 28 days",
            ),
            (
                "2026-04-31",
                "day 31 is out of range: April 2026 has 30 days",
            ),
            (
                "2026-10-30T14:00",
                "date! takes only a date; use ts! for a date with a time and zone",
            ),
            (
                "2026-10-30 14:00",
                "date! takes only a date; use ts! for a date with a time and zone",
            ),
        ];
        for (input, message) in cases {
            assert_eq!(date_err(input), message, "input {input:?}");
        }
    }

    #[test]
    fn time_values() {
        assert_eq!(parse_time("14:30"), Ok((14, 30, 0, 0)));
        assert_eq!(parse_time("00:00:00"), Ok((0, 0, 0, 0)));
        assert_eq!(
            parse_time("23:59:59.999999999"),
            Ok((23, 59, 59, 999_999_999))
        );
        assert_eq!(parse_time("14:30:05.25"), Ok((14, 30, 5, 250_000_000)));
        assert_eq!(parse_time("14:30:05.000000001"), Ok((14, 30, 5, 1)));
    }

    #[test]
    fn time_errors() {
        let cases = [
            ("", r#"empty time; write HH:MM or HH:MM:SS, like "14:30""#),
            ("9:30", r#"write each part with two digits: "09:30""#),
            ("09:5", r#"write each part with two digits: "09:05""#),
            ("14", r#"missing minutes; write HH:MM, like "14:00""#),
            ("1430", r#"add the colons: "14:30""#),
            ("143005", r#"add the colons: "14:30:05""#),
            ("14:30:05:10", "too many parts; write HH:MM:SS"),
            (
                "24:00",
                r#"hour 24 is out of range (00-23); midnight is "00:00""#,
            ),
            ("25:00", "hour 25 is out of range (00-23)"),
            ("14:60", "minute 60 is out of range (00-59)"),
            (
                "23:59:60",
                "second 60 is out of range (00-59); leap seconds aren't supported",
            ),
            (
                "14:30:05,250",
                r#"use "." before the fraction: "14:30:05.250""#,
            ),
            ("14:30:05.", r#"missing digits after ".""#),
            (
                "14:30:05.1234567891",
                "at most 9 fractional digits (nanoseconds)",
            ),
            ("14:30.5", "a fraction needs seconds: write HH:MM:SS.F"),
            ("2:30 PM", r#"use the 24-hour clock: "14:30""#),
            ("12:15am", r#"use the 24-hour clock: "00:15""#),
            ("2:xx PM", r#"use the 24-hour clock HH:MM, like "14:30""#),
            ("2:75 PM", r#"use the 24-hour clock HH:MM, like "14:30""#),
            ("14h30", r#"use the 24-hour clock HH:MM, like "14:30""#),
            (
                "14:30Z",
                r#"time! takes a time of day without a zone; remove "Z", or use ts!"#,
            ),
            (
                "14:30+01:00",
                r#"time! takes a time of day without a zone; remove "+01:00", or use ts!"#,
            ),
            ("T14:30", r#"remove the leading "T": "14:30""#),
            ("14:3x", "minute `3x` is not a number"),
            ("1x:30", "hour `1x` is not a number"),
            ("14:30:0x", "second `0x` is not a number"),
        ];
        for (input, message) in cases {
            assert_eq!(
                parse_time(input).expect_err(input),
                message,
                "input {input:?}"
            );
        }
    }

    fn splitmix(state: &mut u64) -> u64 {
        *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = *state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    #[test]
    fn days_from_civil_oracle() {
        use jiff::tz::TimeZone;
        let mut state = 9u64;
        for _ in 0..20_000 {
            let year = 1 + (splitmix(&mut state) % 9999) as i16;
            let month = 1 + (splitmix(&mut state) % 12) as u8;
            let day = 1 + (splitmix(&mut state) % days_in_month(year, month) as u64) as u8;
            let seconds = jiff::civil::date(year, month as i8, day as i8)
                .to_zoned(TimeZone::UTC)
                .unwrap()
                .timestamp()
                .as_second();
            assert_eq!(
                days_from_civil(year, month, day),
                seconds.div_euclid(86_400),
                "{year}-{month}-{day}"
            );
        }
    }

    #[test]
    fn date_oracle() {
        let mut state = 3u64;
        let mut years: Vec<i16> = (0..2_000)
            .map(|_| 1 + (splitmix(&mut state) % 9999) as i16)
            .collect();
        years.extend([1900, 2000, 2024, 2100]);
        for year in years {
            for month in 0..=13i8 {
                for day in 0..=32i8 {
                    let text = format!("{year:04}-{month:02}-{day:02}");
                    let ours = parse_date(&text).is_ok();
                    let theirs = jiff::civil::Date::new(year, month, day).is_ok();
                    assert_eq!(ours, theirs, "{text}");
                }
            }
        }
    }
}
