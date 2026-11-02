//! `k8s::quantity!`, `cpu!` and `mem!`: Kubernetes resource quantities.

/// Decimal suffixes and their powers of ten.
const DECIMAL: [(&str, i64); 9] = [
    ("n", -9),
    ("u", -6),
    ("m", -3),
    ("k", 3),
    ("M", 6),
    ("G", 9),
    ("T", 12),
    ("P", 15),
    ("E", 18),
];

/// Binary suffixes; `Ki` is 2^10, `Mi` 2^20 and so on.
const BINARY: [&str; 6] = ["Ki", "Mi", "Gi", "Ti", "Pi", "Ei"];

/// The largest magnitude Kubernetes keeps, 2^63 - 1 units, in milli-units.
const LIMIT_MILLIS: u128 = ((1 << 63) - 1) * 1000;

/// A quantity literal's value in milli-units, with its suffix as written.
pub struct Parsed<'a> {
    pub millis: i128,
    pub suffix: &'a str,
}

/// Splits a quantity into its number and suffix as written, checking the grammar.
pub fn split(s: &str) -> Result<(&str, &str), String> {
    if s.is_empty() {
        return Err(
            r#"empty quantity; write a number with an optional suffix, like "500m" or "1.5Gi""#
                .into(),
        );
    }
    if s.starts_with(char::is_whitespace) || s.ends_with(char::is_whitespace) {
        return Err("remove the spaces around the literal".into());
    }
    if s.contains(char::is_whitespace) {
        let joined: String = s.split_whitespace().collect();
        return Err(format!(r#"remove the space: "{joined}""#));
    }
    let unsigned = s.strip_prefix(['+', '-']).unwrap_or(s);
    let int_len = unsigned.bytes().take_while(u8::is_ascii_digit).count();
    let mut end = s.len() - unsigned.len() + int_len;
    let mut digits = int_len;
    if s[end..].starts_with('.') {
        let frac_len = s[end + 1..].bytes().take_while(u8::is_ascii_digit).count();
        end += 1 + frac_len;
        digits += frac_len;
    }
    if digits == 0 {
        return Err(r#"expected a number, like "500m" or "1.5Gi""#.into());
    }
    let (number, suffix) = s.split_at(end);
    check_suffix(number, suffix)?;
    Ok((number, suffix))
}

fn check_suffix(number: &str, suffix: &str) -> Result<(), String> {
    let valid =
        |x: &str| x.is_empty() || BINARY.contains(&x) || DECIMAL.iter().any(|(d, _)| *d == x);
    if valid(suffix) {
        return Ok(());
    }
    // "KB", "MiB", "kb": byte units from other tools.
    let (letter, byte) = match suffix.strip_suffix(['B', 'b']) {
        Some(letter) => (letter, &suffix[letter.len()..]),
        None => (suffix, ""),
    };
    if !byte.is_empty() && valid(letter) && letter != "m" {
        return Err(format!(r#"drop the "{byte}": "{number}{letter}""#));
    }
    let hint = match letter {
        "K" => Some(r#"write "k" (1000) or "Ki" (1024)"#.to_string()),
        "g" | "t" | "p" | "m" if letter != "m" || !byte.is_empty() => {
            let (upper, power) = match letter {
                "m" => ("M", 2),
                "g" => ("G", 3),
                "t" => ("T", 4),
                _ => ("P", 5),
            };
            Some(format!(
                r#"write "{upper}" (10^{}) or "{upper}i" (2^{})"#,
                3 * power,
                10 * power
            ))
        }
        _ => None,
    };
    if let Some(hint) = hint {
        return Err(format!(r#""{suffix}" is not a suffix; {hint}"#));
    }
    if let Some(binary) = BINARY.iter().find(|b| b.eq_ignore_ascii_case(suffix)) {
        return Err(format!(
            r#"unknown suffix "{suffix}"; did you mean "{binary}"?"#
        ));
    }
    if let Some(rest) = suffix.strip_prefix(['e', 'E']) {
        let rest = rest.strip_prefix(['+', '-']).unwrap_or(rest);
        let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
        if rest.is_empty() {
            return Err(r#"missing exponent digits; write e.g. "1e3""#.into());
        }
        if digits == rest.len() {
            return Ok(());
        }
        if digits > 0 {
            return Err(r#"the exponent must be a whole number, like "1e3""#.into());
        }
        // Not an exponent at all, like "Ei1" or "Ex": fall through.
    }
    Err(format!(
        r#"unknown suffix "{suffix}"; use n, u, m, k, M, G, T, P, E, Ki, Mi, Gi, Ti, Pi, Ei or an exponent like e3"#
    ))
}

/// Parses a quantity exactly, rejecting values finer than a milli-unit or beyond
/// 2^63 - 1 units, both of which Kubernetes would silently round or cap.
pub fn parse_quantity(s: &str) -> Result<Parsed<'_>, String> {
    let (number, suffix) = split(s)?;
    let negative = number.starts_with('-');
    let unsigned = number.trim_start_matches(['+', '-']);
    let (int, frac) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    // The value is digits * 2^twos * 10^tens, in units.
    let mut digits: Vec<u8> = int.bytes().chain(frac.bytes()).map(|b| b - b'0').collect();
    let (twos, mut tens) = suffix_scale(suffix);
    tens -= frac.len() as i64;
    let first = digits.iter().position(|&d| d != 0);
    let Some(first) = first else {
        return Ok(Parsed { millis: 0, suffix });
    };
    digits.drain(..first);
    for _ in 0..twos {
        mul_small(&mut digits, 2);
    }
    let out_of_range =
        || format!(r#""{s}" is out of Kubernetes' range (at most 2^63 - 1 in magnitude)"#);
    // Milli-units: shift the decimal point three places further.
    let shift = tens + 3;
    let magnitude = if shift >= 0 {
        if digits.len() as i64 + shift > 40 {
            return Err(out_of_range());
        }
        digits.extend(std::iter::repeat_n(0, shift as usize));
        to_u128(&digits)
            .filter(|&m| m <= LIMIT_MILLIS)
            .ok_or_else(out_of_range)?
    } else {
        let drop = shift.unsigned_abs();
        let keep = digits
            .len()
            .saturating_sub(drop.min(usize::MAX as u64) as usize);
        let exact = digits[keep..].iter().all(|&d| d == 0);
        let floor = to_u128(&digits[..keep])
            .filter(|&m| m <= LIMIT_MILLIS)
            .ok_or_else(out_of_range)?;
        if !exact {
            let ceil = floor + 1;
            if ceil > LIMIT_MILLIS {
                return Err(out_of_range());
            }
            let sign = if negative { "-" } else { "" };
            return Err(format!(
                r#""{s}" is finer than 1m, the smallest step Kubernetes keeps; write "{sign}{ceil}m""#
            ));
        }
        floor
    };
    let millis = magnitude as i128;
    Ok(Parsed {
        millis: if negative { -millis } else { millis },
        suffix,
    })
}

/// The suffix's factor as powers of two and of ten.
fn suffix_scale(suffix: &str) -> (u32, i64) {
    if let Some(i) = BINARY.iter().position(|b| *b == suffix) {
        return (10 * (i as u32 + 1), 0);
    }
    if let Some((_, power)) = DECIMAL.iter().find(|(d, _)| *d == suffix) {
        return (0, *power);
    }
    match suffix.strip_prefix(['e', 'E']) {
        Some(exponent) => {
            let negative = exponent.starts_with('-');
            // Saturate: past 10^±9 the value is far out of range (or zero) either way.
            let size = exponent
                .trim_start_matches(['+', '-'])
                .parse::<i64>()
                .unwrap_or(i64::MAX)
                .min(1_000_000_000);
            (0, if negative { -size } else { size })
        }
        None => (0, 0),
    }
}

/// Multiplies a big-endian decimal digit vector by a small factor.
fn mul_small(digits: &mut Vec<u8>, factor: u32) {
    let mut carry = 0;
    for d in digits.iter_mut().rev() {
        let x = u32::from(*d) * factor + carry;
        *d = (x % 10) as u8;
        carry = x / 10;
    }
    while carry > 0 {
        digits.insert(0, (carry % 10) as u8);
        carry /= 10;
    }
}

/// The value of a decimal digit vector, if it fits.
fn to_u128(digits: &[u8]) -> Option<u128> {
    digits.iter().try_fold(0u128, |acc, &d| {
        acc.checked_mul(10)?.checked_add(u128::from(d))
    })
}

/// `k8s::cpu!`: a quantity that is not negative and has no binary suffix.
pub fn parse_cpu(s: &str) -> Result<i128, String> {
    // Before the precision check, so "-0.1m" isn't told to become "-1m".
    let (number, _) = split(s)?;
    if number.starts_with('-') && number.bytes().any(|b| matches!(b, b'1'..=b'9')) {
        return Err("CPU can't be negative".into());
    }
    let parsed = parse_quantity(s)?;
    if BINARY.contains(&parsed.suffix) {
        return Err(r#"CPU is counted in cores, not bytes; write e.g. "2" or "500m""#.into());
    }
    Ok(parsed.millis)
}

/// `k8s::mem!`: a whole, non-negative number of bytes, without `n`, `u` or `m`.
pub fn parse_mem(s: &str) -> Result<i128, String> {
    let (number, suffix) = split(s)?;
    match suffix {
        "u" | "n" => {
            let name = if suffix == "u" { "micro" } else { "nano" };
            return Err(format!(
                r#""{s}" is a fraction of a byte ("{suffix}" means {name}); write a whole number of bytes"#
            ));
        }
        "m" => {
            // From the number as written, so values finer than 1m get this message too.
            let bytes = thousandths(number);
            return Err(format!(
                r#""{s}" is {bytes} bytes ("m" means milli); did you mean "{number}Mi" or "{number}M"?"#
            ));
        }
        _ => {}
    }
    let millis = parse_quantity(s)?.millis;
    if millis < 0 {
        return Err("a byte count can't be negative".into());
    }
    if millis % 1000 != 0 {
        let rounded = millis / 1000 + 1;
        return Err(format!(
            r#""{s}" is {} bytes; write a whole number of bytes, like "{rounded}""#,
            units(millis)
        ));
    }
    Ok(millis)
}

/// A decimal number as written, divided by 1000: `"500"` → `"0.5"`, `"0.5"` → `"0.0005"`.
fn thousandths(number: &str) -> String {
    let sign = if number.starts_with('-') { "-" } else { "" };
    let unsigned = number.trim_start_matches(['+', '-']);
    let (int, frac) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    let int = format!("{int:0>4}");
    let (int, moved) = int.split_at(int.len() - 3);
    let int = match int.trim_start_matches('0') {
        "" => "0",
        trimmed => trimmed,
    };
    let frac = format!("{moved}{frac}");
    match frac.trim_end_matches('0') {
        "" => format!("{sign}{int}"),
        frac => format!("{sign}{int}.{frac}"),
    }
}

/// Milli-units as a decimal number of units: `1500` → `"1.5"`.
fn units(millis: i128) -> String {
    let sign = if millis < 0 { "-" } else { "" };
    let (int, frac) = (millis.unsigned_abs() / 1000, millis.unsigned_abs() % 1000);
    if frac == 0 {
        format!("{sign}{int}")
    } else {
        let frac = format!("{frac:03}");
        format!("{sign}{int}.{}", frac.trim_end_matches('0'))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn millis(s: &str) -> i128 {
        parse_quantity(s)
            .unwrap_or_else(|e| panic!("{s:?}: {e}"))
            .millis
    }

    #[test]
    fn quantity_values() {
        let exa: i128 = 10i128.pow(18);
        let cases: [(&str, i128); 22] = [
            ("500m", 500),
            ("1", 1000),
            ("1.5Gi", 1_610_612_736_000),
            ("128Mi", 134_217_728_000),
            ("1e3", 1_000_000),
            ("1E3", 1_000_000),
            ("1E+3", 1_000_000),
            ("1e-3", 1),
            ("1E", exa * 1000),
            ("1Ei", (1i128 << 60) * 1000),
            ("+1k", 1_000_000),
            ("-1.5", -1500),
            (".5", 500),
            ("5.", 5000),
            ("1000000n", 1),
            ("0000000000000000000000000001", 1000),
            ("1.000000000000000000000000000000", 1000),
            ("1000000000000000000000000n", exa),
            ("9223372036854775807", 9_223_372_036_854_775_807_000),
            ("0e-400", 0),
            ("7Ei", 7 * (1i128 << 60) * 1000),
            ("-0", 0),
        ];
        for (input, expected) in cases {
            assert_eq!(millis(input), expected, "input {input:?}");
        }
        assert_eq!(split("-1.5Gi"), Ok(("-1.5", "Gi")));
        assert_eq!(parse_quantity("-1.5Gi").unwrap().suffix, "Gi");
    }

    const FINER: &str = "is finer than 1m, the smallest step Kubernetes keeps";
    const RANGE: &str = "is out of Kubernetes' range (at most 2^63 - 1 in magnitude)";

    #[test]
    fn quantity_errors() {
        let number = r#"expected a number, like "500m" or "1.5Gi""#;
        let unknown =
            "use n, u, m, k, M, G, T, P, E, Ki, Mi, Gi, Ti, Pi, Ei or an exponent like e3";
        let exponent = r#"the exponent must be a whole number, like "1e3""#;
        let digits = r#"missing exponent digits; write e.g. "1e3""#;
        let cases = [
            (
                "",
                r#"empty quantity; write a number with an optional suffix, like "500m" or "1.5Gi""#
                    .to_string(),
            ),
            (" 1Gi", "remove the spaces around the literal".into()),
            ("1 Gi", r#"remove the space: "1Gi""#.into()),
            ("Gi", number.into()),
            (".", number.into()),
            ("+", number.into()),
            ("x", number.into()),
            (
                "1K",
                r#""K" is not a suffix; write "k" (1000) or "Ki" (1024)"#.into(),
            ),
            (
                "1g",
                r#""g" is not a suffix; write "G" (10^9) or "Gi" (2^30)"#.into(),
            ),
            (
                "1t",
                r#""t" is not a suffix; write "T" (10^12) or "Ti" (2^40)"#.into(),
            ),
            (
                "1p",
                r#""p" is not a suffix; write "P" (10^15) or "Pi" (2^50)"#.into(),
            ),
            ("1mi", r#"unknown suffix "mi"; did you mean "Mi"?"#.into()),
            ("1KI", r#"unknown suffix "KI"; did you mean "Ki"?"#.into()),
            ("1gi", r#"unknown suffix "gi"; did you mean "Gi"?"#.into()),
            ("1KiB", r#"drop the "B": "1Ki""#.into()),
            ("1MB", r#"drop the "B": "1M""#.into()),
            ("1GiB", r#"drop the "B": "1Gi""#.into()),
            ("1e", digits.into()),
            ("1E+", digits.into()),
            ("1e-", digits.into()),
            ("1e3.5", exponent.into()),
            ("1e3x", exponent.into()),
            ("1.2.3", format!(r#"unknown suffix ".3"; {unknown}"#)),
            ("1µ", format!(r#"unknown suffix "µ"; {unknown}"#)),
            ("0.1m", format!(r#""0.1m" {FINER}; write "1m""#)),
            ("100n", format!(r#""100n" {FINER}; write "1m""#)),
            ("1.0005", format!(r#""1.0005" {FINER}; write "1001m""#)),
            ("-0.1m", format!(r#""-0.1m" {FINER}; write "-1m""#)),
            ("1e-400", format!(r#""1e-400" {FINER}; write "1m""#)),
            ("8Ei", format!(r#""8Ei" {RANGE}"#)),
            ("-8Ei", format!(r#""-8Ei" {RANGE}"#)),
            ("1e400", format!(r#""1e400" {RANGE}"#)),
            (
                "9223372036854775808",
                format!(r#""9223372036854775808" {RANGE}"#),
            ),
            ("1Ei1", format!(r#"unknown suffix "Ei1"; {unknown}"#)),
            ("1Ex", format!(r#"unknown suffix "Ex"; {unknown}"#)),
            (
                "1KB",
                r#""KB" is not a suffix; write "k" (1000) or "Ki" (1024)"#.into(),
            ),
            ("1Kib", r#"drop the "b": "1Ki""#.into()),
            ("1B", r#"drop the "B": "1""#.into()),
            ("1kb", r#"drop the "b": "1k""#.into()),
            (
                "1mb",
                r#""mb" is not a suffix; write "M" (10^6) or "Mi" (2^20)"#.into(),
            ),
        ];
        for (input, message) in cases {
            let got = parse_quantity(input).err();
            assert_eq!(got.as_deref(), Some(message.as_str()), "input {input:?}");
        }
    }

    #[test]
    fn cpu_values() {
        assert_eq!(parse_cpu("500m"), Ok(500));
        assert_eq!(parse_cpu("2"), Ok(2000));
        assert_eq!(parse_cpu("0"), Ok(0));
    }

    #[test]
    fn cpu_errors() {
        let binary = r#"CPU is counted in cores, not bytes; write e.g. "2" or "500m""#;
        let cases = [
            ("-1", "CPU can't be negative".to_string()),
            ("2Gi", binary.into()),
            ("1Ki", binary.into()),
            ("0.1m", format!(r#""0.1m" {FINER}; write "1m""#)),
            ("-0.1m", "CPU can't be negative".into()),
        ];
        for (input, message) in cases {
            assert_eq!(parse_cpu(input), Err(message), "input {input:?}");
        }
    }

    #[test]
    fn mem_values() {
        assert_eq!(parse_mem("512Mi"), Ok(536_870_912_000));
        assert_eq!(parse_mem("1G"), Ok(1_000_000_000_000));
        assert_eq!(parse_mem("0.5Ki"), Ok(512_000));
    }

    #[test]
    fn mem_errors() {
        let cases = [
            ("-1Gi", "a byte count can't be negative"),
            (
                "500m",
                r#""500m" is 0.5 bytes ("m" means milli); did you mean "500Mi" or "500M"?"#,
            ),
            (
                "2000m",
                r#""2000m" is 2 bytes ("m" means milli); did you mean "2000Mi" or "2000M"?"#,
            ),
            (
                "1500m",
                r#""1500m" is 1.5 bytes ("m" means milli); did you mean "1500Mi" or "1500M"?"#,
            ),
            (
                "5u",
                r#""5u" is a fraction of a byte ("u" means micro); write a whole number of bytes"#,
            ),
            (
                "5n",
                r#""5n" is a fraction of a byte ("n" means nano); write a whole number of bytes"#,
            ),
            (
                "1.5",
                r#""1.5" is 1.5 bytes; write a whole number of bytes, like "2""#,
            ),
            (
                "0.1Ki",
                r#""0.1Ki" is 102.4 bytes; write a whole number of bytes, like "103""#,
            ),
            (
                "0.5m",
                r#""0.5m" is 0.0005 bytes ("m" means milli); did you mean "0.5Mi" or "0.5M"?"#,
            ),
            (
                "1.5m",
                r#""1.5m" is 0.0015 bytes ("m" means milli); did you mean "1.5Mi" or "1.5M"?"#,
            ),
        ];
        for (input, message) in cases {
            assert_eq!(
                parse_mem(input),
                Err(message.to_string()),
                "input {input:?}"
            );
        }
    }

    /// Go's exact decimal (`AsDec().String()`) times 1000, if that is a whole number
    /// that fits; otherwise whether it is finer than milli or too large.
    #[derive(Debug, PartialEq)]
    enum Go {
        Millis(i128),
        Finer,
        Huge,
    }

    const MAX_UNITS: u128 = (1 << 63) - 1;

    fn go_millis(decimal: &str) -> Go {
        let (negative, digits) = match decimal.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, decimal),
        };
        let (int, frac) = digits.split_once('.').unwrap_or((digits, ""));
        let frac = frac.trim_end_matches('0');
        let int = int.trim_start_matches('0');
        let units: u128 = if int.is_empty() {
            0
        } else {
            int.parse().unwrap_or(u128::MAX)
        };
        if int.len() > 19 || units > MAX_UNITS || (units == MAX_UNITS && !frac.is_empty()) {
            return Go::Huge;
        }
        if frac.len() > 3 {
            return Go::Finer;
        }
        let int: i128 = if int.is_empty() {
            0
        } else {
            int.parse().unwrap()
        };
        let frac: i128 = format!("{frac:0<3}").parse().unwrap();
        let millis = int * 1000 + frac;
        Go::Millis(if negative { -millis } else { millis })
    }

    /// Our `value()` rule: whole units, rounded away from zero.
    fn value(millis: i128) -> i128 {
        let units = millis / 1000;
        if millis % 1000 == 0 {
            units
        } else {
            units + millis.signum()
        }
    }

    #[test]
    fn quantity_go_oracle() {
        let fixture = include_str!("testdata/k8s_quantity.tsv");
        let mut compared = 0;
        for line in fixture.lines().filter(|l| !l.starts_with('#')) {
            let fields: Vec<&str> = line.split('\t').collect();
            let input = fields[0];
            let ours = parse_quantity(input);
            compared += 1;
            if fields[1] == "err" {
                assert!(ours.is_err(), "Go rejects {input:?}, we accept it");
                continue;
            }
            let go = go_millis(fields[2]);
            match ours {
                Ok(parsed) => {
                    assert_eq!(go, Go::Millis(parsed.millis), "input {input:?}");
                    // Go's Value() is only reliable for non-negative values in range.
                    if parsed.millis >= 0 {
                        assert_eq!(
                            fields[3].parse::<i128>().unwrap(),
                            value(parsed.millis),
                            "Value() of {input:?}"
                        );
                    }
                }
                Err(message) => {
                    let has_digits = input
                        .chars()
                        .take_while(|c| !c.is_ascii_alphabetic())
                        .any(|c| c.is_ascii_digit());
                    let allowed = if message.contains(FINER) {
                        go == Go::Finer
                    } else if message.contains(RANGE) {
                        // Go caps some out-of-range values at exactly 2^63 - 1.
                        let cap = MAX_UNITS as i128 * 1000;
                        go == Go::Huge || go == Go::Millis(cap) || go == Go::Millis(-cap)
                    } else if message.starts_with("expected a number") {
                        !has_digits
                    } else {
                        message == "remove the spaces around the literal"
                    };
                    assert!(allowed, "Go accepts {input:?} ({line}), we say: {message}");
                }
            }
        }
        assert!(compared >= 3000, "only {compared} fixture lines");
    }
}
