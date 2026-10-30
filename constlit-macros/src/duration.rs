//! `dur!`: durations like "1h30m".

use crate::input::{Scale, check_spaces, decimal, unexpected_after};

/// Units from largest to smallest, with their length in nanoseconds and their rank.
const UNITS: [(&str, u128, u8); 9] = [
    ("d", 86_400_000_000_000, 0),
    ("h", 3_600_000_000_000, 1),
    ("m", 60_000_000_000, 2),
    ("s", 1_000_000_000, 3),
    ("ms", 1_000_000, 4),
    ("us", 1_000, 5),
    ("µs", 1_000, 5), // U+00B5 MICRO SIGN
    ("μs", 1_000, 5), // U+03BC GREEK SMALL LETTER MU
    ("ns", 1, 6),
];

const CALENDAR: &[&str] = &[
    "w", "wk", "wks", "week", "weeks", "mo", "month", "months", "y", "yr", "yrs", "year", "years",
];

/// Common spellings and the unit to use instead.
const SPELLINGS: &[(&[&str], &str, &str)] = &[
    (&["hr", "hrs", "hour", "hours"], "h", "hours"),
    (&["min", "mins", "minute", "minutes"], "m", "minutes"),
    (&["sec", "secs", "second", "seconds"], "s", "seconds"),
    (&["msec", "msecs"], "ms", "milliseconds"),
    (&["day", "days"], "d", "days"),
];

const NANOS_PER_SEC: u128 = 1_000_000_000;

fn unit(name: &str) -> Result<(u128, u8), String> {
    if let Some(&(_, nanos, rank)) = UNITS.iter().find(|(unit, ..)| *unit == name) {
        return Ok((nanos, rank));
    }
    if CALENDAR.contains(&name) {
        return Err("weeks, months and years are not supported (months and years have no fixed length); write days instead, like \"14d\"".into());
    }
    if let Some((_, unit, meaning)) = SPELLINGS.iter().find(|(names, ..)| names.contains(&name)) {
        return Err(format!("unknown unit `{name}`; use `{unit}` for {meaning}"));
    }
    Err(format!(
        "unknown unit `{name}`; valid units are ns, us, µs, ms, s, m, h, d"
    ))
}

/// Parses a duration literal into whole seconds and remaining nanoseconds.
pub fn parse_duration(s: &str) -> Result<(u64, u32), String> {
    const TOO_LARGE: &str = "duration too large for Duration";
    if s.is_empty() {
        return Err("empty duration; write a number and a unit, like \"30s\"".into());
    }
    check_spaces(s)?;
    let mut rest = s;
    let mut total: u128 = 0;
    let mut previous: Option<(&str, u8)> = None;
    loop {
        if rest.starts_with('-') {
            return Err("durations can't be negative".into());
        }
        let Some(number) = decimal(rest)? else {
            return Err("expected a number, like \"30s\"".into());
        };
        rest = &rest[number.text.len()..];
        rest = rest.strip_prefix(' ').unwrap_or(rest);
        let unit_len = rest
            .char_indices()
            .find(|(_, c)| !c.is_alphabetic())
            .map_or(rest.len(), |(i, _)| i);
        if unit_len == 0 {
            return Err(match rest.chars().next() {
                Some(c) if !c.is_ascii_digit() => unexpected_after(number.text, c),
                _ => format!(
                    "missing unit after {0}; write e.g. \"{0}s\" or \"{0}ms\"",
                    number.text
                ),
            });
        }
        let name = &rest[..unit_len];
        rest = &rest[unit_len..];
        let (unit_nanos, rank) = unit(name)?;
        if let Some((previous_name, previous_rank)) = previous {
            if rank == previous_rank {
                return Err(format!(
                    "units must go from largest to smallest, each at most once: `{name}` twice"
                ));
            }
            if rank < previous_rank {
                return Err(format!(
                    "units must go from largest to smallest, each at most once: `{name}` after `{previous_name}`"
                ));
            }
        }
        previous = Some((name, rank));
        let nanos = match number.times(unit_nanos) {
            Ok(nanos) => nanos,
            Err(Scale::TooLarge) => return Err(TOO_LARGE.into()),
            Err(Scale::NotWhole) => {
                return Err(format!(
                    "{}{name} is not a whole number of nanoseconds",
                    number.text
                ));
            }
        };
        total = total.checked_add(nanos).ok_or(TOO_LARGE)?;
        if rest.is_empty() {
            break;
        }
        rest = rest.strip_prefix(' ').unwrap_or(rest);
    }
    let secs = u64::try_from(total / NANOS_PER_SEC).map_err(|_| TOO_LARGE)?;
    Ok((secs, (total % NANOS_PER_SEC) as u32))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn err(s: &str) -> String {
        parse_duration(s).expect_err(s)
    }

    #[test]
    fn dur_values() {
        assert_eq!(parse_duration("1h30m"), Ok((5400, 0)));
        assert_eq!(parse_duration("1h 30m"), Ok((5400, 0)));
        assert_eq!(parse_duration("1.5h"), Ok((5400, 0)));
        assert_eq!(parse_duration("500ms"), Ok((0, 500_000_000)));
        assert_eq!(parse_duration("2m30s"), Ok((150, 0)));
        assert_eq!(parse_duration("1d 12h"), Ok((129600, 0)));
        assert_eq!(parse_duration("250us"), Ok((0, 250_000)));
        assert_eq!(parse_duration("250µs"), Ok((0, 250_000)));
        assert_eq!(parse_duration("7ns"), Ok((0, 7)));
        assert_eq!(parse_duration("0s"), Ok((0, 0)));
        assert_eq!(parse_duration("0.5s"), Ok((0, 500_000_000)));
        assert_eq!(parse_duration("30 s"), Ok((30, 0)));
        assert_eq!(parse_duration("1h1m1s1ms1us1ns"), Ok((3661, 1_001_001)));
    }

    #[test]
    fn dur_errors() {
        assert_eq!(
            err(""),
            "empty duration; write a number and a unit, like \"30s\""
        );
        assert_eq!(
            err("30"),
            "missing unit after 30; write e.g. \"30s\" or \"30ms\""
        );
        assert_eq!(
            err("1h30"),
            "missing unit after 30; write e.g. \"30s\" or \"30ms\""
        );
        assert_eq!(err("5hr"), "unknown unit `hr`; use `h` for hours");
        assert_eq!(err("5 hours"), "unknown unit `hours`; use `h` for hours");
        assert_eq!(err("5min"), "unknown unit `min`; use `m` for minutes");
        assert_eq!(err("5secs"), "unknown unit `secs`; use `s` for seconds");
        assert_eq!(
            err("5msec"),
            "unknown unit `msec`; use `ms` for milliseconds"
        );
        assert_eq!(err("5days"), "unknown unit `days`; use `d` for days");
        assert_eq!(
            err("5x"),
            "unknown unit `x`; valid units are ns, us, µs, ms, s, m, h, d"
        );
        let calendar = "weeks, months and years are not supported (months and years have no fixed length); write days instead, like \"14d\"";
        for s in [
            "2w", "1mo", "1y", "1wk", "2weeks", "1month", "1yr", "3years",
        ] {
            assert_eq!(err(s), calendar, "{s}");
        }
        assert_eq!(
            err("30m1h"),
            "units must go from largest to smallest, each at most once: `h` after `m`"
        );
        assert_eq!(
            err("1h2h"),
            "units must go from largest to smallest, each at most once: `h` twice"
        );
        assert_eq!(err("-5s"), "durations can't be negative");
        assert_eq!(err("1.5ns"), "1.5ns is not a whole number of nanoseconds");
        assert_eq!(
            err("999999999999999999999d"),
            "duration too large for Duration"
        );
        assert_eq!(err("h"), "expected a number, like \"30s\"");
    }

    #[test]
    fn dur_decimal_edges() {
        assert_eq!(err(".5s"), "write a digit before the point, like \"0.5s\"");
        assert_eq!(
            err("1.s"),
            "write a digit after the point, like \"1s\" or \"1.5s\""
        );
        assert_eq!(
            err("1..5s"),
            "write a digit after the point, like \"1s\" or \"1.5s\""
        );
        assert_eq!(
            parse_duration("1.500000000000000000000000000000s"),
            Ok((1, 500_000_000))
        );
    }

    #[test]
    fn suggestions_follow_input() {
        assert_eq!(err(".5h"), "write a digit before the point, like \"0.5h\"");
        assert_eq!(
            err("1h .5m"),
            "write a digit before the point, like \"0.5m\""
        );
        assert_eq!(
            err("2.d"),
            "write a digit after the point, like \"2d\" or \"2.5d\""
        );
        assert_eq!(
            err("1..5h"),
            "write a digit after the point, like \"1h\" or \"1.5h\""
        );
    }

    #[test]
    fn separators() {
        assert_eq!(
            err("1_000ms"),
            "digit separators like `_` aren't supported; write the number without them"
        );
        assert_eq!(
            err("1'000s"),
            "digit separators like `'` aren't supported; write the number without them"
        );
        assert_eq!(err("1,5h"), "unexpected `,` after 1");
        assert_eq!(
            err("30 5s"),
            "missing unit after 30; write e.g. \"30s\" or \"30ms\""
        );
    }

    #[test]
    fn whitespace_rules() {
        assert_eq!(err(" 1h"), "remove the spaces around the literal");
        assert_eq!(err("1h  30m"), "use single spaces between parts");
    }

    #[test]
    fn dur_max() {
        assert_eq!(parse_duration("18446744073709551615s"), Ok((u64::MAX, 0)));
        assert_eq!(
            err("18446744073709551616s"),
            "duration too large for Duration"
        );
    }

    #[test]
    fn dur_round_trip() {
        let mut state = 0x2545_f491_4f6c_dd1du64;
        let mut next = || {
            state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^ (z >> 31)
        };
        for _ in 0..2000 {
            let secs = next() % (1u64 << 40);
            let nanos = (next() % 1_000_000_000) as u32;
            let (d, h, m, s) = (secs / 86400, secs / 3600 % 24, secs / 60 % 60, secs % 60);
            let mut text = String::new();
            for (n, unit) in [(d, "d"), (h, "h"), (m, "m"), (s, "s"), (nanos as u64, "ns")] {
                if n != 0 {
                    text += &format!("{n}{unit}");
                }
            }
            if text.is_empty() {
                text = "0s".into();
            }
            assert_eq!(parse_duration(&text), Ok((secs, nanos)), "{text}");
        }
    }
}
