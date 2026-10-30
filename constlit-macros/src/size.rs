//! `size!`: byte sizes like "64 KiB", with strict units.

use crate::input::{Scale, check_spaces, decimal, unexpected_after};

/// Prefixes for powers 1..=5: (SI unit, IEC unit, Kubernetes unit).
const PREFIXES: [(&str, &str, &str); 5] = [
    ("KB", "KiB", "Ki"),
    ("MB", "MiB", "Mi"),
    ("GB", "GiB", "Gi"),
    ("TB", "TiB", "Ti"),
    ("PB", "PiB", "Pi"),
];

/// Bytes per `unit`, for exact unit spellings only.
fn unit(unit: &str) -> Result<u128, String> {
    if unit.is_empty() || unit == "B" {
        return Ok(1);
    }
    for (power, (si, iec, kubernetes)) in (1u32..).zip(PREFIXES) {
        if unit == si {
            return Ok(1000u128.pow(power));
        }
        if unit == iec || unit == kubernetes {
            return Ok(1024u128.pow(power));
        }
        let lower = unit.to_ascii_lowercase();
        if [si, iec, kubernetes]
            .iter()
            .any(|u| u.to_ascii_lowercase() == lower)
        {
            return Err(format!(
                "`{unit}` is ambiguous: write `{iec}` ({} bytes) or `{si}` ({} bytes)",
                1024u128.pow(power),
                1000u128.pow(power)
            ));
        }
    }
    Err(format!(
        "unknown unit `{unit}`; use B, KB/MB/GB/TB/PB (powers of 1000) or KiB/MiB/GiB/TiB/PiB (powers of 1024)"
    ))
}

/// Parses a byte-size literal into a number of bytes.
pub fn parse_size(s: &str) -> Result<u64, String> {
    const TOO_LARGE: &str = "size too large for u64";
    if s.is_empty() {
        return Err("empty size; write a number and a unit, like \"64 KiB\"".into());
    }
    check_spaces(s)?;
    if s.starts_with('-') {
        return Err("sizes can't be negative".into());
    }
    let Some(number) = decimal(s)? else {
        return Err("expected a number, like \"64 KiB\"".into());
    };
    let rest = &s[number.text.len()..];
    let rest = rest.strip_prefix(' ').unwrap_or(rest);
    if let Some(c) = rest.chars().next().filter(|c| !c.is_alphabetic()) {
        return Err(unexpected_after(number.text, c));
    }
    let bytes_per_unit = unit(rest)?;
    let bytes = match number.times(bytes_per_unit) {
        Ok(bytes) => bytes,
        Err(Scale::TooLarge) => return Err(TOO_LARGE.into()),
        Err(Scale::NotWhole) => return Err(format!("{s} is not a whole number of bytes")),
    };
    u64::try_from(bytes).map_err(|_| TOO_LARGE.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn err(s: &str) -> String {
        parse_size(s).expect_err(s)
    }

    #[test]
    fn size_values() {
        assert_eq!(parse_size("64 KiB"), Ok(65536));
        assert_eq!(parse_size("64KiB"), Ok(65536));
        assert_eq!(parse_size("64 KB"), Ok(64000));
        assert_eq!(parse_size("1.5 KiB"), Ok(1536));
        assert_eq!(parse_size("128Mi"), Ok(134217728));
        assert_eq!(parse_size("4096"), Ok(4096));
        assert_eq!(parse_size("4096 B"), Ok(4096));
        assert_eq!(parse_size("1 PiB"), Ok(1125899906842624));
        assert_eq!(parse_size("2 GB"), Ok(2_000_000_000));
        assert_eq!(parse_size("1 Ti"), Ok(1 << 40));
        assert_eq!(parse_size("0"), Ok(0));
    }

    #[test]
    fn size_errors() {
        assert_eq!(
            err("64 kb"),
            "`kb` is ambiguous: write `KiB` (1024 bytes) or `KB` (1000 bytes)"
        );
        assert_eq!(
            err("1 mb"),
            "`mb` is ambiguous: write `MiB` (1048576 bytes) or `MB` (1000000 bytes)"
        );
        assert_eq!(
            err("1 KIB"),
            "`KIB` is ambiguous: write `KiB` (1024 bytes) or `KB` (1000 bytes)"
        );
        assert_eq!(
            err("1 kB"),
            "`kB` is ambiguous: write `KiB` (1024 bytes) or `KB` (1000 bytes)"
        );
        assert_eq!(
            err("1 gi"),
            "`gi` is ambiguous: write `GiB` (1073741824 bytes) or `GB` (1000000000 bytes)"
        );
        assert_eq!(
            err("1 XB"),
            "unknown unit `XB`; use B, KB/MB/GB/TB/PB (powers of 1000) or KiB/MiB/GiB/TiB/PiB (powers of 1024)"
        );
        assert_eq!(err("1.3 B"), "1.3 B is not a whole number of bytes");
        assert_eq!(
            err("1.0001 KiB"),
            "1.0001 KiB is not a whole number of bytes"
        );
        assert_eq!(err("-1 KiB"), "sizes can't be negative");
        assert_eq!(
            err(""),
            "empty size; write a number and a unit, like \"64 KiB\""
        );
        assert_eq!(err("KiB"), "expected a number, like \"64 KiB\"");
        assert_eq!(
            err(".5 KiB"),
            "write a digit before the point, like \"0.5 KiB\""
        );
        assert_eq!(
            err("1. KiB"),
            "write a digit after the point, like \"1 KiB\" or \"1.5 KiB\""
        );
    }

    #[test]
    fn suggestions_follow_input() {
        assert_eq!(
            err(".5 GiB"),
            "write a digit before the point, like \"0.5 GiB\""
        );
        assert_eq!(
            err("2. MB"),
            "write a digit after the point, like \"2 MB\" or \"2.5 MB\""
        );
        assert_eq!(
            err("2.MB"),
            "write a digit after the point, like \"2MB\" or \"2.5MB\""
        );
        assert_eq!(
            err("1_000 KiB"),
            "digit separators like `_` aren't supported; write the number without them"
        );
    }

    #[test]
    fn size_u64_edges() {
        assert_eq!(parse_size("18446744073709551615"), Ok(u64::MAX));
        assert_eq!(err("18446744073709551616"), "size too large for u64");
        assert_eq!(err("16384 PiB"), "size too large for u64");
        assert_eq!(parse_size("16383 PiB"), Ok(16383 << 50));
    }

    #[test]
    fn whitespace_rules() {
        assert_eq!(err("64 KiB "), "remove the spaces around the literal");
        assert_eq!(err("64  KiB"), "use single spaces between parts");
    }
}
