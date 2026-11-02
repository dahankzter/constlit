//! `systemd::timespan!`: systemd time spans, like `"5min 30s"`.
//!
//! The units and their values are systemd's (`parse_sec`); the fixture in
//! `testdata/systemd_timespan.tsv` pins them against `systemd-analyze timespan`.
//! Beyond systemd: a bare number (whose unit depends on the setting), `M` (month,
//! easily read as `m`), values finer than 1us (which systemd truncates),
//! `infinity`, `+` and surrounding spaces are errors.

use crate::suggest;

const SEC: u64 = 1_000_000;

/// The whitespace systemd skips between parts.
const SPACE: [char; 4] = [' ', '\t', '\n', '\r'];
const MONTH: u64 = 2_629_800 * SEC;
const YEAR: u64 = 31_557_600 * SEC;

/// Unit names and their length in microseconds.
const UNITS: [(&str, u64); 30] = [
    ("us", 1),
    ("usec", 1),
    ("\u{b5}s", 1),
    ("\u{3bc}s", 1),
    ("ms", 1000),
    ("msec", 1000),
    ("s", SEC),
    ("sec", SEC),
    ("second", SEC),
    ("seconds", SEC),
    ("m", 60 * SEC),
    ("min", 60 * SEC),
    ("minute", 60 * SEC),
    ("minutes", 60 * SEC),
    ("h", 3600 * SEC),
    ("hr", 3600 * SEC),
    ("hour", 3600 * SEC),
    ("hours", 3600 * SEC),
    ("d", 86_400 * SEC),
    ("day", 86_400 * SEC),
    ("days", 86_400 * SEC),
    ("w", 604_800 * SEC),
    ("week", 604_800 * SEC),
    ("weeks", 604_800 * SEC),
    ("M", MONTH),
    ("month", MONTH),
    ("months", MONTH),
    ("y", YEAR),
    ("year", YEAR),
    ("years", YEAR),
];

/// Parses a time span into seconds and nanoseconds.
pub fn parse_timespan(s: &str) -> Result<(u64, u32), String> {
    if s.is_empty() {
        return Err(r#"empty time span; write e.g. "5min 30s""#.into());
    }
    if s.starts_with(char::is_whitespace) || s.ends_with(char::is_whitespace) {
        return Err("remove the spaces around the literal".into());
    }
    // Zero is zero in any unit, so a bare "0" is not ambiguous.
    if s == "0" {
        return Ok((0, 0));
    }
    if s.contains("infinity") {
        return Err("infinity has no Duration; use Duration::MAX".into());
    }
    if s.starts_with('-') {
        return Err("time spans can't be negative".into());
    }
    if s.contains('+') {
        let fixed = s.replace('+', "");
        parse_timespan(&fixed)?;
        return Err(format!(r#"write the span without "+": "{fixed}""#));
    }
    let mut total: u128 = 0;
    let mut rest = s;
    loop {
        rest = rest.trim_start_matches(SPACE);
        if rest.is_empty() {
            break;
        }
        let int_len = rest.bytes().take_while(u8::is_ascii_digit).count();
        let (int, after) = rest.split_at(int_len);
        let (frac, after) = match after.strip_prefix('.') {
            Some(after) => {
                let frac_len = after.bytes().take_while(u8::is_ascii_digit).count();
                if frac_len == 0 {
                    return Err(format!(r#"missing digits after "." in "{s}""#));
                }
                after.split_at(frac_len)
            }
            None => ("", after),
        };
        if int.is_empty() && frac.is_empty() {
            let c = rest.chars().next().expect("not empty");
            return Err(if c.is_alphabetic() {
                format!(r#"missing number before "{c}"; write e.g. "5{c}""#)
            } else if c == ',' {
                format!(
                    r#"unexpected "," in "{s}"; separate parts with spaces or nothing, like "1h 30min""#
                )
            } else {
                format!(r#"unexpected "{c}" in "{s}""#)
            });
        }
        let number = &rest[..int.len() + if frac.is_empty() { 0 } else { frac.len() + 1 }];
        let after = after.trim_start_matches(SPACE);
        let unit_len: usize = after
            .chars()
            .take_while(|c| c.is_alphabetic())
            .map(char::len_utf8)
            .sum();
        let (unit, after) = after.split_at(unit_len);
        if unit.is_empty() {
            return Err(match after.chars().next() {
                None => format!(
                    r#"missing unit after {number}; write e.g. "{number}s" or "{number}min""#
                ),
                Some(c) if c.is_ascii_digit() || SPACE.contains(&c) => {
                    format!(
                        r#"missing unit after {number}; write e.g. "{number}s" or "{number}min""#
                    )
                }
                Some(',') => format!(
                    r#"unexpected "," in "{s}"; separate parts with spaces or nothing, like "1h 30min""#
                ),
                Some(c) => format!(r#"unexpected "{c}" in "{s}""#),
            });
        }
        if unit == "M" {
            return Err(format!(
                r#""M" means month here and reads like "m" (minute); write "{number}month" or "{number}min""#
            ));
        }
        let Some(&(_, per)) = UNITS.iter().find(|(name, _)| *name == unit) else {
            return Err(
                match suggest::closest(unit, UNITS.iter().map(|(name, _)| *name)) {
                    Some(near) => format!(r#"unknown unit "{unit}"; did you mean "{near}"?"#),
                    None => {
                        format!(r#"unknown unit "{unit}"; use us, ms, s, min, h, d, w, month or y"#)
                    }
                },
            );
        };
        // number * per, exactly, in microseconds, within systemd's limits: an integer
        // part of at most i64::MAX, below (2^64 - 1) / unit, and a total below 2^64 - 1.
        let too_long = || format!(r#""{s}" is too long for systemd"#);
        let frac = frac.trim_end_matches('0');
        let int_value = if int.is_empty() {
            Ok(0)
        } else {
            int.parse::<u64>()
        };
        let Ok(int_value) = int_value else {
            return Err(too_long());
        };
        if int_value > i64::MAX as u64 || int_value >= u64::MAX / per {
            return Err(too_long());
        }
        let digits = format!("{int}{frac}");
        let digits = digits.trim_start_matches('0');
        if digits.len() > 38 {
            return Err(too_long());
        }
        let scaled = u128::from(per)
            .checked_mul(digits.parse::<u128>().unwrap_or(0))
            .ok_or_else(too_long)?;
        let divisor = 10u128.pow(frac.len() as u32);
        if scaled % divisor != 0 {
            return Err(format!(
                r#""{s}" is finer than 1us, the smallest step systemd keeps"#
            ));
        }
        total += scaled / divisor;
        if total >= u128::from(u64::MAX) {
            return Err(too_long());
        }
        rest = after;
    }
    let micros = total as u64;
    Ok((micros / SEC, (micros % SEC) as u32 * 1000))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn micros(s: &str) -> u64 {
        let (secs, nanos) = parse_timespan(s).unwrap_or_else(|e| panic!("{s:?}: {e}"));
        secs * 1_000_000 + u64::from(nanos) / 1000
    }

    #[test]
    fn timespan_values() {
        assert_eq!(micros("5min 30s"), 330_000_000);
        assert_eq!(micros("1.5h"), 5_400_000_000);
        assert_eq!(micros("1h30min"), 5_400_000_000);
        assert_eq!(micros("2 weeks"), 1_209_600_000_000);
        assert_eq!(micros("1month"), 2_629_800_000_000);
        assert_eq!(micros("1y"), 31_557_600_000_000);
        assert_eq!(micros("500ms"), 500_000);
        assert_eq!(micros("1us"), 1);
        assert_eq!(micros("1m"), 60_000_000);
        assert_eq!(micros(".5s"), 500_000);
        assert_eq!(micros("0"), 0);
        assert_eq!(micros("1.0000000000000000000000000000000s"), 1_000_000);
        assert_eq!(micros("1\ts"), 1_000_000);
        assert_eq!(micros("1h\t30min"), 5_400_000_000);
    }

    #[test]
    fn timespan_errors() {
        let cases = [
            ("", r#"empty time span; write e.g. "5min 30s""#),
            (" 5s", "remove the spaces around the literal"),
            (
                "30",
                r#"missing unit after 30; write e.g. "30s" or "30min""#,
            ),
            (
                "1h 30",
                r#"missing unit after 30; write e.g. "30s" or "30min""#,
            ),
            (
                "1M",
                r#""M" means month here and reads like "m" (minute); write "1month" or "1min""#,
            ),
            (
                "2 M",
                r#""M" means month here and reads like "m" (minute); write "2month" or "2min""#,
            ),
            (
                "1.0000001s",
                r#""1.0000001s" is finer than 1us, the smallest step systemd keeps"#,
            ),
            (
                "0.5us",
                r#""0.5us" is finer than 1us, the smallest step systemd keeps"#,
            ),
            ("infinity", "infinity has no Duration; use Duration::MAX"),
            (
                "5 parsecs",
                r#"unknown unit "parsecs"; use us, ms, s, min, h, d, w, month or y"#,
            ),
            ("5mins", r#"unknown unit "mins"; did you mean "min"?"#),
            (
                "1h,30min",
                r#"unexpected "," in "1h,30min"; separate parts with spaces or nothing, like "1h 30min""#,
            ),
            ("-5s", "time spans can't be negative"),
            ("+5s", r#"write the span without "+": "5s""#),
            ("5.s", r#"missing digits after "." in "5.s""#),
            ("1_000s", r#"unexpected "_" in "1_000s""#),
            (
                "999999999999y",
                r#""999999999999y" is too long for systemd"#,
            ),
            (
                "123456789012345678901234567890week",
                r#""123456789012345678901234567890week" is too long for systemd"#,
            ),
            (
                "18446744073709551615us",
                r#""18446744073709551615us" is too long for systemd"#,
            ),
            ("584542y", r#""584542y" is too long for systemd"#),
        ];
        for (input, message) in cases {
            assert_eq!(
                parse_timespan(input),
                Err(message.to_string()),
                "input {input:?}"
            );
        }
    }

    #[test]
    fn timespan_oracle() {
        let fixture = include_str!("testdata/systemd_timespan.tsv");
        let (mut compared, mut accepted) = (0, 0);
        for line in fixture.lines().filter(|l| !l.starts_with('#')) {
            let fields: Vec<&str> = line.split('\t').collect();
            let input = fields[0];
            compared += 1;
            let ours = parse_timespan(input);
            if let Err(message) = &ours
                && let Some(start) = message.rfind(": \"")
            {
                let fixed = message[start + 3..].trim_end_matches('"');
                assert!(
                    parse_timespan(fixed).is_ok(),
                    "{input:?}: suggestion {fixed:?}"
                );
            }
            match (fields[1], ours) {
                ("ok", Ok((secs, nanos))) => {
                    accepted += 1;
                    let us = secs * 1_000_000 + u64::from(nanos) / 1000;
                    assert_eq!(us.to_string(), fields[2], "input {input:?}");
                }
                ("err", Err(_)) => {}
                ("ok", Err(message)) => {
                    accepted += 1;
                    let allowed = message.starts_with("missing unit after")
                        || (message.starts_with(r#""M" means month"#) && input.contains('M'))
                        || (message.contains("finer than 1us") && input.contains('.'))
                        || (message.starts_with("infinity") && input.contains("infinity"))
                        || (message.starts_with(r#"write the span without "+""#)
                            && input.contains('+'))
                        || (message == "remove the spaces around the literal"
                            && input.trim() != input);
                    assert!(allowed, "systemd accepts {input:?}, we say: {message}");
                }
                (_, Ok(_)) => panic!("systemd rejects {input:?}, we accept it"),
                _ => unreachable!(),
            }
        }
        assert!(
            compared >= 2000 && accepted >= 1000,
            "{compared} lines, {accepted} accepted"
        );
    }
}
