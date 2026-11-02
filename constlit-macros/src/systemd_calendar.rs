//! `systemd::calendar!`: systemd calendar expressions, normalized as systemd does.
//!
//! The grammar and normal form are systemd's (`calendarspec.c`); the fixture in
//! `testdata/systemd_calendar.tsv` pins them against `systemd-analyze calendar`.
//! Narrowed: "UTC" is the only time zone, and fractional seconds, `@` timestamps
//! and two-digit years are errors.

/// A parsed expression: bit sets for matching, and systemd's normalized form.
/// Bits index values directly (years from 1970).
pub struct Calendar {
    pub weekdays: u8,
    pub years: [u64; 4],
    pub months: u16,
    pub days: u32,
    pub from_end: bool,
    pub hours: u32,
    pub minutes: u64,
    pub seconds: u64,
    pub utc: bool,
    pub normalized: String,
}

/// One entry of a comma-separated chain: a value, a range, and a repetition step.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Entry {
    start: u32,
    stop: Option<u32>,
    repeat: u32,
    /// Steps count down (`~07/1`: 7, 6, ... days from the end), not up.
    down: bool,
}

struct Field {
    name: &'static str,
    min: u32,
    max: u32,
    width: usize,
}

const YEAR: Field = Field {
    name: "year",
    min: 1970,
    max: 2199,
    width: 4,
};
const MONTH: Field = Field {
    name: "month",
    min: 1,
    max: 12,
    width: 2,
};
const DAY: Field = Field {
    name: "day",
    min: 1,
    max: 31,
    width: 2,
};
/// Days counted back from the month's end (`~`), which systemd caps at 28.
const DAY_FROM_END: Field = Field {
    name: "day from the end",
    min: 1,
    max: 28,
    width: 2,
};
const HOUR: Field = Field {
    name: "hour",
    min: 0,
    max: 23,
    width: 2,
};
const MINUTE: Field = Field {
    name: "minute",
    min: 0,
    max: 59,
    width: 2,
};
const SECOND: Field = Field {
    name: "second",
    min: 0,
    max: 59,
    width: 2,
};

const WEEKDAYS: [(&str, &str); 7] = [
    ("Mon", "monday"),
    ("Tue", "tuesday"),
    ("Wed", "wednesday"),
    ("Thu", "thursday"),
    ("Fri", "friday"),
    ("Sat", "saturday"),
    ("Sun", "sunday"),
];

/// systemd's shorthands, as the expressions they stand for.
const SHORTHANDS: [(&str, &str); 9] = [
    ("minutely", "*-*-* *:*:00"),
    ("hourly", "*-*-* *:00:00"),
    ("daily", "*-*-* 00:00:00"),
    ("weekly", "Mon *-*-* 00:00:00"),
    ("monthly", "*-*-01 00:00:00"),
    ("yearly", "*-01-01 00:00:00"),
    ("annually", "*-01-01 00:00:00"),
    ("quarterly", "*-01,04,07,10-01 00:00:00"),
    ("semiannually", "*-01,07-01 00:00:00"),
];

/// Parses a calendar expression.
pub fn parse_calendar(s: &str) -> Result<Calendar, String> {
    if s.is_empty() {
        return Err(
            r#"empty calendar expression; write e.g. "Mon..Fri *-*-* 09:00" or "daily""#.into(),
        );
    }
    if s.starts_with(char::is_whitespace) || s.ends_with(char::is_whitespace) {
        return Err("remove the spaces around the literal".into());
    }
    if s.starts_with('@') {
        return Err(
            r#""@" timestamps aren't calendar expressions; use constlit::ts! for a fixed time"#
                .into(),
        );
    }
    let mut tokens: Vec<&str> = s.split(' ').filter(|t| !t.is_empty()).collect();
    let utc = tokens.len() > 1 && tokens[tokens.len() - 1].eq_ignore_ascii_case("utc");
    let shorthand = |word: &str| {
        SHORTHANDS
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(word))
    };
    if utc {
        // systemd takes exactly one space before the zone.
        let before = &s[..s.len() - 3];
        if before.ends_with("  ") {
            let fixed = format!("{} {}", before.trim_end_matches(' '), &s[s.len() - 3..]);
            parse_calendar(&fixed)?;
            return Err(format!(r#"use one space before "UTC": "{fixed}""#));
        }
        tokens.pop();
    } else if let Some(last) = tokens.last()
        && tokens.len() > 1
        && shorthand(last).is_none()
        && (last.contains('/') && last.starts_with(|c: char| c.is_ascii_alphabetic())
            || (2..=5).contains(&last.len()) && last.bytes().all(|b| b.is_ascii_uppercase()))
    {
        return Err(r#"calendar! takes "UTC" only; for a time zone, call next_after with a zoned date-time"#.into());
    }
    if let [word] = tokens.as_slice()
        && let Some((_, expansion)) = shorthand(word)
    {
        let expansion = if utc {
            format!("{expansion} UTC")
        } else {
            expansion.to_string()
        };
        return parse_calendar(&expansion);
    }
    let mut rest = tokens.as_slice();
    let mut weekdays = 0x7f;
    let mut weekday_text = None;
    if let Some(first) = rest.first()
        && first.starts_with(|c: char| c.is_ascii_alphabetic())
    {
        weekdays = parse_weekdays(first)?;
        // Every weekday is no restriction, and systemd leaves it out.
        if weekdays != 0x7f {
            weekday_text = Some(format_weekdays(weekdays));
        }
        rest = &rest[1..];
    }
    let (mut date, mut time) = (None, None);
    for token in rest {
        if token.contains(':') && time.is_none() {
            time = Some(*token);
        } else if (token.contains('-') || token.contains('~')) && date.is_none() && time.is_none() {
            date = Some(*token);
        } else {
            return Err(format!(
                r#"expected a date (like "*-*-01") or a time (like "12:00"), found "{token}""#
            ));
        }
    }
    let (years, months, days, from_end) = match date {
        Some(date) => parse_date(s, date)?,
        None => (Vec::new(), Vec::new(), Vec::new(), false),
    };
    let (hours, minutes, seconds) = match time {
        Some(time) => parse_time(time)?,
        None => (zero(), zero(), zero()),
    };
    let mut normalized = String::new();
    if let Some(text) = weekday_text {
        normalized.push_str(&text);
        normalized.push(' ');
    }
    normalized.push_str(&format!(
        "{}-{}{}{} {}:{}:{}",
        format_chain(&years, &YEAR),
        format_chain(&months, &MONTH),
        if from_end { "~" } else { "-" },
        format_chain(&days, &DAY),
        format_chain(&hours, &HOUR),
        format_chain(&minutes, &MINUTE),
        format_chain(&seconds, &SECOND),
    ));
    if utc {
        normalized.push_str(" UTC");
    }
    let mut year_bits = [0u64; 4];
    for year in values(&years, &YEAR) {
        let i = (year - YEAR.min) as usize;
        year_bits[i / 64] |= 1 << (i % 64);
    }
    let mask = |entries: &[Entry], field: &Field| {
        values(entries, field).fold(0u64, |bits, v| bits | 1 << v)
    };
    Ok(Calendar {
        weekdays,
        years: year_bits,
        months: mask(&months, &MONTH) as u16,
        days: mask(&days, &DAY) as u32,
        from_end,
        hours: mask(&hours, &HOUR) as u32,
        minutes: mask(&minutes, &MINUTE),
        seconds: mask(&seconds, &SECOND),
        utc,
        normalized,
    })
}

fn zero() -> Vec<Entry> {
    vec![Entry {
        start: 0,
        stop: None,
        repeat: 0,
        down: false,
    }]
}

type Date = (Vec<Entry>, Vec<Entry>, Vec<Entry>, bool);

/// `[year-]month-day`, or `[year-]month~day` counting days from the month's end.
fn parse_date(s: &str, date: &str) -> Result<Date, String> {
    let (left, day, from_end) = match date.split_once('~') {
        Some((left, day)) => (left, day, true),
        None => match date.rsplit_once('-') {
            Some((left, day)) => (left, day, false),
            None => unreachable!("dates contain - or ~"),
        },
    };
    let (year, month) = match left.split_once('-') {
        Some((year, month)) => (Some(year), month),
        None => (None, left),
    };
    if year.is_none() && !from_end && left.len() == 4 && left.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!(
            r#"a date is [year-]month-day; add the day, like "{left}-{day}-01""#
        ));
    }
    if year == Some("") {
        return Err(format!(r#"empty year in "{date}""#));
    }
    if let Some(year) = year
        && year.bytes().any(|b| b.is_ascii_digit())
        && year
            .split([',', '.', '/'])
            .all(|n| n.is_empty() || (n.len() <= 2 && n.bytes().all(|b| b.is_ascii_digit())))
    {
        let four: String = year
            .split_inclusive([',', '.', '/'])
            .map(|part| {
                let (n, sep) = part.split_at(part.trim_end_matches([',', '.', '/']).len());
                match n.parse::<u32>() {
                    Ok(n)
                        if !n.to_string().is_empty() && !year.contains('/')
                            || !sep.starts_with('/') =>
                    {
                        let full = if n < 70 { 2000 + n } else { 1900 + n };
                        format!("{full}{sep}")
                    }
                    _ => part.to_string(),
                }
            })
            .collect();
        let fixed = s.replacen(year, &four, 1);
        // The fixed text has four-digit years, so checking it can't come back here.
        // When it doesn't parse either, the normal error below says why.
        if fixed != s && parse_calendar(&fixed).is_ok() {
            return Err(format!(r#"write the year with all four digits: "{fixed}""#));
        }
    }
    let years = match year {
        Some(year) => parse_chain(year, &YEAR)?,
        None => Vec::new(),
    };
    let days = if from_end {
        // systemd's rules for lists after "~" aren't regular, so one day, a range or
        // one open step ("~07/1": 7, 6, ... 1 days from the end) is supported.
        let unsupported = r#"with "~", give one day, a range or a step, like "*-02~03", "*-*~01..07" or "*-*~07/1""#;
        if day.contains(',') || day == "*" || day.contains('/') && day.contains("..") {
            return Err(unsupported.into());
        }
        match day.split_once('/') {
            Some((start, step)) => {
                let start = number(start, &DAY_FROM_END)?;
                let step = match step.parse::<u32>() {
                    Ok(n) if n > 0 && step.bytes().all(|b| b.is_ascii_digit()) => n,
                    _ => return Err(format!(r#"the step in "{day}" must be a positive number"#)),
                };
                if step >= start {
                    return Err(format!(
                        r#""~{day}" never repeats: the next day from the end, {}, is before the last day"#,
                        i64::from(start) - i64::from(step)
                    ));
                }
                vec![Entry {
                    start,
                    stop: None,
                    repeat: step,
                    down: true,
                }]
            }
            None => parse_chain(day, &DAY_FROM_END)?,
        }
    } else {
        parse_chain(day, &DAY)?
    };
    Ok((years, parse_chain(month, &MONTH)?, days, from_end))
}

/// `hour:minute[:second]`.
type Time = (Vec<Entry>, Vec<Entry>, Vec<Entry>);

fn parse_time(time: &str) -> Result<Time, String> {
    let parts: Vec<&str> = time.split(':').collect();
    if parts.len() > 3 {
        return Err(format!(r#"a time is hour:minute[:second], found "{time}""#));
    }
    let seconds = match parts.get(2) {
        Some(second) if second.replace("..", "").contains('.') => {
            return Err("fractional seconds aren't supported; write whole seconds".into());
        }
        Some(second) => parse_chain(second, &SECOND)?,
        None => zero(),
    };
    Ok((
        parse_chain(parts[0], &HOUR)?,
        parse_chain(parts[1], &MINUTE)?,
        seconds,
    ))
}

/// A comma-separated chain; `*` is the empty chain (every value).
fn parse_chain(text: &str, field: &Field) -> Result<Vec<Entry>, String> {
    if text == "*" {
        return Ok(Vec::new());
    }
    let mut entries = Vec::new();
    for item in text.split(',') {
        if item.is_empty() {
            return Err(format!(r#"empty {} in "{text}""#, field.name));
        }
        let (range, repeat) = match item.split_once('/') {
            Some((range, repeat)) => (range, Some(repeat)),
            None => (item, None),
        };
        if range == "*" && repeat.is_none() {
            return Err(r#""*" can't be part of a list; write "*" alone or list values"#.into());
        }
        if range == "*" {
            return Err(format!(
                r#"a repetition needs a start, like "{:0w$}/{}""#,
                field.min,
                repeat.unwrap_or("2"),
                w = field.width
            ));
        }
        let repeat = match repeat {
            Some(r) if !r.is_empty() && r.bytes().all(|b| b.is_ascii_digit()) => {
                match r.parse::<u64>() {
                    // systemd keeps steps in an int.
                    Ok(n) if n > i32::MAX as u64 => {
                        return Err(format!(r#"the step in "{item}" is too large"#));
                    }
                    Ok(n) if n > 0 => n as u32,
                    _ => return Err(format!(r#"the step in "{item}" must be a positive number"#)),
                }
            }
            Some(_) => return Err(format!(r#"the step in "{item}" must be a positive number"#)),
            None => 0,
        };
        let (start, stop) = match range.split_once("..") {
            Some((a, b)) => (number(a, field)?, Some(number(b, field)?)),
            None => (number(range, field)?, None),
        };
        // systemd rejects an open repetition that never reaches a second value.
        if stop.is_none() && repeat > 0 && start + repeat > field.max {
            return Err(format!(
                r#""{item}" never repeats: the next {}, {}, is past {:0w$}"#,
                field.name,
                start + repeat,
                field.max,
                w = field.width
            ));
        }
        let mut stop = stop;
        if let Some(end) = stop {
            if end < start {
                return Err(format!(r#"the range "{range}" ends before it starts"#));
            }
            // systemd ends a stepped range at the last value it reaches.
            if let Some(steps) = (end - start).checked_div(repeat) {
                stop = Some(start + steps * repeat);
            }
        }
        // A range stepped by 1 is just the range; an open "13/1" keeps its step.
        let repeat = if stop.is_some() && repeat == 1 {
            0
        } else {
            repeat
        };
        let entry = if stop == Some(start) {
            Entry {
                start,
                stop: None,
                repeat: 0,
                down: false,
            }
        } else {
            Entry {
                start,
                stop,
                repeat,
                down: false,
            }
        };
        entries.push(entry);
    }
    entries.sort();
    entries.dedup();
    Ok(entries)
}

fn number(text: &str, field: &Field) -> Result<u32, String> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!("{} `{text}` is not a number", field.name));
    }
    match text.parse::<u32>() {
        Ok(n) if (field.min..=field.max).contains(&n) => Ok(n),
        _ => Err(format!(
            "{} {text} is out of range ({:0w$}-{:0w$})",
            field.name,
            field.min,
            field.max,
            w = field.width
        )),
    }
}

/// Every value a chain matches.
fn values<'a>(entries: &'a [Entry], field: &'a Field) -> Box<dyn Iterator<Item = u32> + 'a> {
    if entries.is_empty() {
        return Box::new(field.min..=field.max);
    }
    Box::new(
        entries
            .iter()
            .flat_map(move |e| -> Box<dyn Iterator<Item = u32>> {
                if e.down {
                    let (start, step) = (e.start, e.repeat);
                    return Box::new(
                        (0..=(start - field.min) / step).map(move |i| start - i * step),
                    );
                }
                let end = e
                    .stop
                    .unwrap_or(if e.repeat > 0 { field.max } else { e.start });
                Box::new((e.start..=end).step_by(e.repeat.max(1) as usize))
            }),
    )
}

fn format_chain(entries: &[Entry], field: &Field) -> String {
    if entries.is_empty() {
        return "*".into();
    }
    let w = field.width;
    entries
        .iter()
        .map(|e| {
            let mut text = format!("{:0w$}", e.start);
            if let Some(stop) = e.stop {
                text.push_str(&format!("..{stop:0w$}"));
            }
            if e.repeat > 0 {
                text.push_str(&format!("/{}", e.repeat));
            }
            text
        })
        .collect::<Vec<_>>()
        .join(",")
}

/// Weekday names (any case, short or long), lists, and ranges with `..` or `-`.
fn parse_weekdays(text: &str) -> Result<u8, String> {
    let day = |name: &str| {
        let lower = name.to_ascii_lowercase();
        WEEKDAYS
            .iter()
            .position(|(short, long)| lower == short.to_ascii_lowercase() || lower == *long)
            .ok_or_else(|| format!(r#"unknown weekday or shorthand "{name}""#))
    };
    let mut mask = 0u8;
    for item in text.split(',') {
        let range = item.split_once("..").or_else(|| item.split_once('-'));
        match range {
            Some((a, b)) => {
                let (a, b) = (day(a)?, day(b)?);
                if b < a {
                    let days: Vec<&str> = (a..7).chain(0..=b).map(|i| WEEKDAYS[i].0).collect();
                    return Err(format!(
                        r#"weekday ranges go forward; write "{}""#,
                        days.join(",")
                    ));
                }
                for i in a..=b {
                    mask |= 1 << i;
                }
            }
            None => mask |= 1 << day(item)?,
        }
    }
    Ok(mask)
}

/// systemd's weekday form: runs of three or more as `A..B`, others listed.
fn format_weekdays(mask: u8) -> String {
    let mut parts = Vec::new();
    let mut i = 0;
    while i < 7 {
        if mask & 1 << i == 0 {
            i += 1;
            continue;
        }
        let start = i;
        while i < 7 && mask & 1 << i != 0 {
            i += 1;
        }
        let end = i - 1;
        match end - start {
            0 => parts.push(WEEKDAYS[start].0.to_string()),
            1 => parts.push(format!("{},{}", WEEKDAYS[start].0, WEEKDAYS[end].0)),
            _ => parts.push(format!("{}..{}", WEEKDAYS[start].0, WEEKDAYS[end].0)),
        }
    }
    parts.join(",")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bits(values: &[u32]) -> u64 {
        values.iter().fold(0, |b, v| b | 1 << v)
    }

    #[test]
    fn calendar_values() {
        let c = parse_calendar("Mon..Fri *-*-* 09:00").unwrap();
        assert_eq!(c.weekdays, 0b001_1111);
        assert_eq!(c.hours, 1 << 9);
        assert_eq!(c.minutes, 1);
        assert_eq!(c.seconds, 1);
        assert_eq!(c.months, bits(&(1..=12).collect::<Vec<_>>()) as u16);
        assert_eq!(c.normalized, "Mon..Fri *-*-* 09:00:00");
        let q = parse_calendar("quarterly").unwrap();
        assert_eq!(q.months, bits(&[1, 4, 7, 10]) as u16);
        assert_eq!(q.days, 1 << 1);
        assert_eq!(q.normalized, "*-01,04,07,10-01 00:00:00");
        let e = parse_calendar("*-02~03").unwrap();
        assert!(e.from_end);
        assert_eq!(e.days, 1 << 3);
        assert_eq!(
            parse_calendar("12:00 utc").unwrap().normalized,
            "*-*-* 12:00:00 UTC"
        );
        assert!(parse_calendar("12:00 UTC").unwrap().utc);
        assert_eq!(
            parse_calendar("*-*-01..31/7 00:00").unwrap().normalized,
            "*-*-01..29/7 00:00:00"
        );
        let y = parse_calendar("2026/2-01-01").unwrap();
        assert_eq!(y.years[0] >> (2026 - 1970) & 1, 1);
        assert_eq!(y.years[0] >> (2027 - 1970) & 1, 0);
    }

    #[test]
    fn calendar_more_values() {
        let n = |s: &str| {
            parse_calendar(s)
                .unwrap_or_else(|e| panic!("{s:?}: {e}"))
                .normalized
        };
        assert_eq!(n("*:*:10..20"), "*-*-* *:*:10..20");
        assert_eq!(n("12:00:00..30"), "*-*-* 12:00:00..30");
        assert_eq!(n("Mon..Sun"), "*-*-* 00:00:00");
        assert_eq!(n("monday-sun 12:00 UTC"), "*-*-* 12:00:00 UTC");
        assert_eq!(n("*-*~07/1"), "*-*~07/1 00:00:00");
        assert_eq!(n("Mon *-05~07/1"), "Mon *-05~07/1 00:00:00");
        assert_eq!(n("Daily"), "*-*-* 00:00:00");
        assert_eq!(n("Mon  12:00"), "Mon *-*-* 12:00:00");
        // ~10/3: 10, 7, 4 and 1 days from the end.
        let c = parse_calendar("*-*~10/3").unwrap();
        assert_eq!(c.days, (1 << 10) | (1 << 7) | (1 << 4) | (1 << 1));
    }

    #[test]
    fn calendar_errors() {
        let cases = [
            (
                "",
                r#"empty calendar expression; write e.g. "Mon..Fri *-*-* 09:00" or "daily""#,
            ),
            (" daily", "remove the spaces around the literal"),
            (
                "Sat..Mon",
                r#"weekday ranges go forward; write "Sat,Sun,Mon""#,
            ),
            ("25:00", "hour 25 is out of range (00-23)"),
            ("*:60", "minute 60 is out of range (00-59)"),
            ("*-13-01", "month 13 is out of range (01-12)"),
            ("*-*-32", "day 32 is out of range (01-31)"),
            ("1969-01-01", "year 1969 is out of range (1970-2199)"),
            (
                "12:00:00.5",
                "fractional seconds aren't supported; write whole seconds",
            ),
            (
                "Wed 12:00 Europe/Stockholm",
                r#"calendar! takes "UTC" only; for a time zone, call next_after with a zoned date-time"#,
            ),
            (
                "@1700000000",
                r#""@" timestamps aren't calendar expressions; use constlit::ts! for a fixed time"#,
            ),
            (
                "26-01-01",
                r#"write the year with all four digits: "2026-01-01""#,
            ),
            ("*/2:00", r#"a repetition needs a start, like "00/2""#),
            ("today", r#"unknown weekday or shorthand "today""#),
            ("-01-01", r#"empty year in "-01-01""#),
            (",-01-01", r#"empty year in ",""#),
            ("5/-01-01", r#"the step in "5/" must be a positive number"#),
            ("Mon -*-01 12:00", r#"empty year in "-*-01""#),
            (
                "*-*-01/4294967295",
                r#"the step in "01/4294967295" is too large"#,
            ),
            (
                "1..5/2147483648:00",
                r#"the step in "1..5/2147483648" is too large"#,
            ),
            (
                "*-*~03/3",
                r#""~03/3" never repeats: the next day from the end, 0, is before the last day"#,
            ),
            (
                "*-*~1,2",
                r#"with "~", give one day, a range or a step, like "*-02~03", "*-*~01..07" or "*-*~07/1""#,
            ),
            ("12:00  UTC", r#"use one space before "UTC": "12:00 UTC""#),
            (
                "12:00 GMT",
                r#"calendar! takes "UTC" only; for a time zone, call next_after with a zoned date-time"#,
            ),
            (
                "2026-02",
                r#"a date is [year-]month-day; add the day, like "2026-02-01""#,
            ),
            (
                "*,5:00",
                r#""*" can't be part of a list; write "*" alone or list values"#,
            ),
        ];
        for (input, message) in cases {
            assert_eq!(
                parse_calendar(input).err().as_deref(),
                Some(message),
                "input {input:?}"
            );
        }
    }

    /// Random strings, multi-byte ones included, never panic or recurse forever.
    #[test]
    fn parsers_never_panic() {
        let alphabet: Vec<char> = "0123456789*-~:,./ MonSatUTCdailyµé\tM5".chars().collect();
        let mut state = 11u64;
        let mut next = || {
            state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^ (z >> 31)
        };
        for _ in 0..50_000 {
            let len = next() % 14;
            let s: String = (0..len)
                .map(|_| alphabet[(next() % alphabet.len() as u64) as usize])
                .collect();
            let _ = parse_calendar(&s);
            let _ = crate::systemd_timespan::parse_timespan(&s);
            let _ = crate::systemd_unit::check_unit(&s);
        }
    }

    #[test]
    fn calendar_oracle() {
        let fixture = include_str!("testdata/systemd_calendar.tsv");
        let (mut compared, mut accepted) = (0, 0);
        for line in fixture.lines().filter(|l| !l.starts_with('#')) {
            let fields: Vec<&str> = line.split('\t').collect();
            let input = fields[0];
            compared += 1;
            let ours = parse_calendar(input);
            if let Err(message) = &ours
                && let Some(start) = message.rfind(": \"")
            {
                let fixed = message[start + 3..].trim_end_matches('"');
                assert!(
                    parse_calendar(fixed).is_ok(),
                    "{input:?}: suggestion {fixed:?}"
                );
            }
            match (fields[1], ours) {
                ("ok", Ok(c)) => {
                    accepted += 1;
                    assert_eq!(c.normalized, fields[2], "input {input:?}");
                }
                ("err", Err(_)) => {}
                ("ok", Err(message)) => {
                    accepted += 1;
                    let allowed = (message.starts_with("write the year with all four digits"))
                        || (message.starts_with(r#"with "~", give one day"#)
                            && input.contains('~'))
                        || (message.starts_with("fractional seconds")
                            && input.replace("..", "").contains('.'))
                        || (message.starts_with(r#"calendar! takes "UTC" only"#)
                            && input.split(' ').next_back().is_some_and(|z| {
                                z.chars()
                                    .all(|c| c.is_ascii_alphabetic() || c == '/' || c == '_')
                            }))
                        || message.starts_with(r#""@" timestamps"#)
                        || (message == "remove the spaces around the literal"
                            && input.trim() != input);
                    assert!(allowed, "systemd accepts {input:?}, we say: {message}");
                }
                (_, Ok(c)) => panic!(
                    "systemd rejects {input:?}, we accept it as {}",
                    c.normalized
                ),
                _ => unreachable!(),
            }
        }
        assert!(
            compared >= 2000 && accepted >= 1000,
            "{compared} lines, {accepted} accepted"
        );
    }
}
