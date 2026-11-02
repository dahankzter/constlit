//! `cron!`: standard five-field cron expressions and the `@` shortcuts.

/// A decoded cron expression: one bit per allowed value in each field.
#[derive(Debug, PartialEq, Eq)]
pub struct Cron {
    pub minutes: u64,
    pub hours: u32,
    /// Bit d for day d, in 1..=31.
    pub days: u32,
    /// Bit m for month m, in 1..=12.
    pub months: u16,
    /// Bit w for weekday w, in 0..=6 with Sunday = 0.
    pub weekdays: u8,
    /// The day-of-month field starts with `*`.
    pub dom_star: bool,
    /// The day-of-week field starts with `*`.
    pub dow_star: bool,
    /// The minute or hour field starts with `*`.
    pub time_star: bool,
}

const SHORTCUTS: [(&str, &str); 7] = [
    ("@yearly", "0 0 1 1 *"),
    ("@annually", "0 0 1 1 *"),
    ("@monthly", "0 0 1 * *"),
    ("@weekly", "0 0 * * 0"),
    ("@daily", "0 0 * * *"),
    ("@midnight", "0 0 * * *"),
    ("@hourly", "0 * * * *"),
];

const MONTHS: [&str; 12] = [
    "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC",
];
const DAYS: [&str; 7] = ["SUN", "MON", "TUE", "WED", "THU", "FRI", "SAT"];

const NONSTANDARD: &str = "`L`, `W` and `#` are not standard cron and aren't supported";

/// One of the five fields: its names for messages, its range, and its value names.
struct Field {
    /// As in "minute 60 is out of range".
    name: &'static str,
    /// As in "empty item in minute field".
    field: &'static str,
    min: u32,
    max: u32,
    /// The largest value `*` and `a/n` reach; differs from `max` only for day of week.
    star_max: u32,
    names: &'static [&'static str],
    /// The value of `names[0]`.
    first_name: u32,
}

const FIELDS: [Field; 5] = [
    Field {
        name: "minute",
        field: "minute",
        min: 0,
        max: 59,
        star_max: 59,
        names: &[],
        first_name: 0,
    },
    Field {
        name: "hour",
        field: "hour",
        min: 0,
        max: 23,
        star_max: 23,
        names: &[],
        first_name: 0,
    },
    Field {
        name: "day of month",
        field: "day-of-month",
        min: 1,
        max: 31,
        star_max: 31,
        names: &[],
        first_name: 0,
    },
    Field {
        name: "month",
        field: "month",
        min: 1,
        max: 12,
        star_max: 12,
        names: &MONTHS,
        first_name: 1,
    },
    Field {
        name: "day of week",
        field: "day-of-week",
        min: 0,
        max: 7,
        star_max: 6,
        names: &DAYS,
        first_name: 0,
    },
];

/// Parses a five-field cron expression or an `@` shortcut.
pub fn parse_cron(s: &str) -> Result<Cron, String> {
    if s.is_empty() {
        return Err(
            r#"empty cron expression; write 5 fields like "*/5 * * * *" or a shortcut like "@daily""#
                .into(),
        );
    }
    if s.starts_with(char::is_whitespace) || s.ends_with(char::is_whitespace) {
        return Err("remove the spaces around the literal".into());
    }
    if s.contains(['\n', '\r']) {
        return Err("line break in cron expression; write it on one line".into());
    }
    if s.starts_with('@') {
        return shortcut(s);
    }
    let fields: Vec<&str> = s.split([' ', '\t']).filter(|f| !f.is_empty()).collect();
    if fields.len() != 5 {
        let hint = match fields.len() {
            6 => "; seconds fields aren't supported",
            7 => "; Quartz's seconds and year fields aren't supported",
            _ => "",
        };
        return Err(format!(
            "expected 5 fields (minute hour day-of-month month day-of-week), found {}{hint}",
            fields.len()
        ));
    }
    let mut masks = [0u64; 5];
    for ((text, field), mask) in fields.iter().zip(&FIELDS).zip(&mut masks) {
        *mask = parse_field(text, field)?;
    }
    // Day of week 7 is Sunday, like 0.
    let weekdays = (masks[4] | masks[4] >> 7) & 0x7f;
    Ok(Cron {
        minutes: masks[0],
        hours: masks[1] as u32,
        days: masks[2] as u32,
        months: masks[3] as u16,
        weekdays: weekdays as u8,
        dom_star: fields[2].starts_with('*'),
        dow_star: fields[4].starts_with('*'),
        time_star: fields[0].starts_with('*') || fields[1].starts_with('*'),
    })
}

fn shortcut(s: &str) -> Result<Cron, String> {
    let word = s.split([' ', '\t']).next().unwrap_or(s);
    if word == "@reboot" {
        return Err("@reboot runs at startup, not on a schedule, so it has no CronSpec".into());
    }
    match SHORTCUTS.iter().find(|(name, _)| *name == word) {
        Some(_) if word != s => Err(format!(
            "a shortcut stands alone; remove what follows `{word}`"
        )),
        Some((_, expansion)) => parse_cron(expansion),
        None => Err(format!(
            "unknown shortcut `{word}`; use @yearly, @annually, @monthly, @weekly, @daily, @midnight or @hourly"
        )),
    }
}

/// Parses a `,`-list of items into a mask with bit v for each value v.
fn parse_field(text: &str, field: &Field) -> Result<u64, String> {
    if let Some(c) = text
        .chars()
        .find(|c| !(c.is_ascii_alphanumeric() || "*-/,".contains(*c)))
    {
        return Err(match c {
            '?' => "`?` is Quartz syntax; use `*`".into(),
            '#' => NONSTANDARD.into(),
            _ => format!("invalid character `{c}` in {} field", field.field),
        });
    }
    let mut mask = 0;
    for item in text.split(',') {
        if item.is_empty() {
            return Err(format!("empty item in {} field", field.field));
        }
        mask |= parse_item(item, field)?;
    }
    Ok(mask)
}

/// Parses `*`, `*/n`, `a`, `a-b`, `a-b/n` or `a/n` (meaning `a-max/n`).
fn parse_item(item: &str, field: &Field) -> Result<u64, String> {
    let (range, step) = match item.split_once('/') {
        Some((range, step)) => (range, Some(step)),
        None => (item, None),
    };
    let step = match step {
        None => 1,
        Some(step) if !step.is_empty() && step.bytes().all(|b| b.is_ascii_digit()) => {
            match step.parse::<u32>() {
                Ok(0) => return Err("step `/0` must be at least 1".into()),
                Ok(n) => n,
                Err(_) => u32::MAX,
            }
        }
        Some(step) => return Err(format!("step `/{step}` is not a number")),
    };
    let (low, high) = if range == "*" {
        (field.min, field.star_max)
    } else if let Some((a, b)) = range.split_once('-') {
        let (low, high) = (value(a, field)?, value(b, field)?);
        if low > high {
            return Err(format!(
                "range `{range}` goes backwards; cron ranges don't wrap around"
            ));
        }
        (low, high)
    } else {
        let low = value(range, field)?;
        (
            low,
            if step > 1 || item.contains('/') {
                field.star_max.max(low)
            } else {
                low
            },
        )
    };
    Ok((low..=high)
        .step_by(step as usize)
        .fold(0, |mask, v| mask | 1 << v))
}

/// One number or name in `field`.
fn value(text: &str, field: &Field) -> Result<u32, String> {
    if text.is_empty() {
        return Err(format!("missing number in {} field", field.field));
    }
    if text.contains('*') {
        return Err(format!(
            "`*` can't be part of `{text}`; write `*` alone or a number"
        ));
    }
    if text.bytes().all(|b| b.is_ascii_digit()) {
        return match text.parse::<u32>() {
            Ok(v) if (field.min..=field.max).contains(&v) => Ok(v),
            _ => Err(format!(
                "{} {text} is out of range ({}-{})",
                field.name, field.min, field.max
            )),
        };
    }
    // Quartz's `L`, `LW`, `15W`, `5L`, in either case.
    let letters = text.trim_start_matches(|c: char| c.is_ascii_digit());
    if ["L", "W", "LW"]
        .iter()
        .any(|q| letters.eq_ignore_ascii_case(q))
    {
        return Err(NONSTANDARD.into());
    }
    if field.names.is_empty() {
        return Err("names are only allowed in the month and day-of-week fields".into());
    }
    if let Some(i) = field
        .names
        .iter()
        .position(|n| n.eq_ignore_ascii_case(text))
    {
        return Ok(field.first_name + i as u32);
    }
    Err(match field.names.len() {
        12 => format!("unknown month name `{text}`; use JAN-DEC"),
        _ => format!("unknown day name `{text}`; use SUN-SAT"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(s: &str) -> Cron {
        parse_cron(s).unwrap_or_else(|e| panic!("{s:?}: {e}"))
    }

    fn err(s: &str) -> String {
        parse_cron(s)
            .err()
            .unwrap_or_else(|| panic!("{s:?} parsed"))
    }

    fn bits(values: impl IntoIterator<Item = u32>) -> u64 {
        values.into_iter().fold(0, |acc, v| acc | 1 << v)
    }

    const ALL_MINUTES: u64 = (1 << 60) - 1;
    const ALL_HOURS: u32 = (1 << 24) - 1;
    const ALL_DAYS: u32 = !1;
    const ALL_MONTHS: u16 = 0b1_1111_1111_1110;
    const ALL_WEEKDAYS: u8 = 0b111_1111;

    #[test]
    fn cron_values() {
        let c = ok("*/15 * * * *");
        assert_eq!(c.minutes, bits([0, 15, 30, 45]));
        assert_eq!(c.hours, ALL_HOURS);
        assert_eq!(
            (c.days, c.months, c.weekdays),
            (ALL_DAYS, ALL_MONTHS, ALL_WEEKDAYS)
        );
        assert!(c.dom_star && c.dow_star && c.time_star);

        let c = ok("0 9-17 * * MON-FRI");
        assert!(!c.time_star);
        assert_eq!(c.minutes, 1);
        assert_eq!(u64::from(c.hours), bits(9..=17));
        assert_eq!(u64::from(c.weekdays), bits(1..=5));
        assert!(c.dom_star && !c.dow_star);

        let c = ok("0 0 1,15 * *");
        assert_eq!(u64::from(c.days), bits([1, 15]));
        assert!(!c.dom_star && c.dow_star);

        assert_eq!(ok("5/20 * * * *").minutes, bits([5, 25, 45]));
        assert_eq!(ok("1-10/4 * * * *").minutes, bits([1, 5, 9]));
        assert_eq!(u64::from(ok("0 0 * * 5-7").weekdays), bits([5, 6, 0]));
        assert_eq!(ok("0 0 * * 7").weekdays, 1);
        assert_eq!(u64::from(ok("0 0 * * 1/2").weekdays), bits([1, 3, 5]));
        assert_eq!(u64::from(ok("0 0 * jan,Jul *").months), bits([1, 7]));
        assert_eq!(u64::from(ok("0 0 * * sun,Sat").weekdays), bits([0, 6]));
        assert_eq!(ok("0\t0  * *\t*").minutes, 1);
        assert_eq!(ok("59 23 31 12 *").minutes, 1 << 59);
        assert_eq!(ok("* * * * *").minutes, ALL_MINUTES);
        assert!(ok("0 * * * *").time_star);
        assert!(ok("*/30 2 * * *").time_star);
        assert!(ok("@hourly").time_star);
        assert!(!ok("0-59/5 2 * * *").time_star);
        assert!(!ok("@daily").time_star);
    }

    #[test]
    fn shortcuts() {
        let cases = [
            ("@yearly", "0 0 1 1 *"),
            ("@annually", "0 0 1 1 *"),
            ("@monthly", "0 0 1 * *"),
            ("@weekly", "0 0 * * 0"),
            ("@daily", "0 0 * * *"),
            ("@midnight", "0 0 * * *"),
            ("@hourly", "0 * * * *"),
        ];
        for (shortcut, expansion) in cases {
            assert_eq!(ok(shortcut), ok(expansion), "{shortcut}");
        }
    }

    #[test]
    fn cron_errors() {
        let cases = [
            (
                "",
                r#"empty cron expression; write 5 fields like "*/5 * * * *" or a shortcut like "@daily""#,
            ),
            (" * * * * *", "remove the spaces around the literal"),
            (
                "* * * *",
                "expected 5 fields (minute hour day-of-month month day-of-week), found 4",
            ),
            ("60 * * * *", "minute 60 is out of range (0-59)"),
            ("0 24 * * *", "hour 24 is out of range (0-23)"),
            ("0 0 0 * *", "day of month 0 is out of range (1-31)"),
            ("0 0 32 * *", "day of month 32 is out of range (1-31)"),
            ("0 0 * 13 *", "month 13 is out of range (1-12)"),
            ("0 0 * * 8", "day of week 8 is out of range (0-7)"),
            (
                "5-1 * * * *",
                "range `5-1` goes backwards; cron ranges don't wrap around",
            ),
            (
                "0 0 * * FRI-MON",
                "range `FRI-MON` goes backwards; cron ranges don't wrap around",
            ),
            ("*/0 * * * *", "step `/0` must be at least 1"),
            ("0 0 * FOO *", "unknown month name `FOO`; use JAN-DEC"),
            ("0 0 * * Funday", "unknown day name `Funday`; use SUN-SAT"),
            (
                "0 MON * * *",
                "names are only allowed in the month and day-of-week fields",
            ),
            (
                "@reboot",
                "@reboot runs at startup, not on a schedule, so it has no CronSpec",
            ),
            (
                "@every 5m",
                "unknown shortcut `@every`; use @yearly, @annually, @monthly, @weekly, @daily, @midnight or @hourly",
            ),
            (
                "@Daily",
                "unknown shortcut `@Daily`; use @yearly, @annually, @monthly, @weekly, @daily, @midnight or @hourly",
            ),
            ("1,,2 * * * *", "empty item in minute field"),
            ("1,2, * * * *", "empty item in minute field"),
            ("0 0 * * 1.5", "invalid character `.` in day-of-week field"),
            (
                "x * * * *",
                "names are only allowed in the month and day-of-week fields",
            ),
            ("% * * * *", "invalid character `%` in minute field"),
            (
                "0 0 * *\n*",
                "line break in cron expression; write it on one line",
            ),
            (
                "0 0\r\n* * *",
                "line break in cron expression; write it on one line",
            ),
            (
                "hello * * * *",
                "names are only allowed in the month and day-of-week fields",
            ),
            (
                "*-5 * * * *",
                "`*` can't be part of `*`; write `*` alone or a number",
            ),
        ];
        for (input, message) in cases {
            assert_eq!(err(input), message, "input {input:?}");
        }
    }

    #[test]
    fn cron_dialect_errors() {
        let nonstandard = "`L`, `W` and `#` are not standard cron and aren't supported";
        let cases = [
            (
                "0 * * * * *",
                "expected 5 fields (minute hour day-of-month month day-of-week), found 6; seconds fields aren't supported",
            ),
            ("0 0 ? * *", "`?` is Quartz syntax; use `*`"),
            ("0 0 * * ?", "`?` is Quartz syntax; use `*`"),
            ("0 0 L * *", nonstandard),
            ("0 0 15W * *", nonstandard),
            ("0 0 LW * *", nonstandard),
            ("0 0 * * 5#3", nonstandard),
            ("0 0 * * 5L", nonstandard),
            ("0 0 l * *", nonstandard),
            ("0 0 15w * *", nonstandard),
            ("0 0 * * 5l", nonstandard),
            ("0 0 * * l", nonstandard),
            (
                "0 0 0 1 * ? 2026",
                "expected 5 fields (minute hour day-of-month month day-of-week), found 7; Quartz's seconds and year fields aren't supported",
            ),
        ];
        for (input, message) in cases {
            assert_eq!(err(input), message, "input {input:?}");
        }
    }

    /// The croner oracle in `constlit/tests/cron_next.rs` builds specs from the
    /// generator's masks; this checks `parse_cron` reads the same text the same way.
    #[test]
    fn cron_generator_masks() {
        use crate::cron_gen::field;
        let mut state = 17u64;
        for _ in 0..20_000 {
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
            let mask = |i: usize| fields[i].1;
            let expected = Cron {
                minutes: mask(0),
                hours: mask(1) as u32,
                days: mask(2) as u32,
                months: mask(3) as u16,
                weekdays: mask(4) as u8,
                dom_star: fields[2].0 == "*",
                dow_star: fields[4].0 == "*",
                time_star: fields[0].0.starts_with('*') || fields[1].0.starts_with('*'),
            };
            assert_eq!(ok(&expr), expected, "{expr}");
        }
    }
}
