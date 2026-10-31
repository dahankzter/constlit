//! `version!`: SemVer 2.0.0 versions, exactly as <https://semver.org> defines them.

const NAMES: [&str; 3] = ["major", "minor", "patch"];

/// Parses a SemVer 2.0.0 version into `(major, minor, patch, pre, build)`.
/// `pre` and `build` are stored without their `-` or `+`, and are empty when absent.
pub fn parse_version(s: &str) -> Result<(u64, u64, u64, String, String), String> {
    if s.is_empty() {
        return Err(r#"empty version; write MAJOR.MINOR.PATCH, like "1.2.3""#.into());
    }
    if s.starts_with(char::is_whitespace) || s.ends_with(char::is_whitespace) {
        return Err("remove the spaces around the literal".into());
    }
    if let Some(rest) = s.strip_prefix(['v', 'V']) {
        // Git tags spell versions as "v1.2.3"; SemVer itself doesn't.
        parse_version(rest)?;
        return Err(format!(r#"remove the leading "v"; write "{rest}""#));
    }
    let (rest, build) = match s.split_once('+') {
        Some((rest, build)) => (rest, Some(build)),
        None => (s, None),
    };
    let (core, pre) = match rest.split_once('-') {
        Some((core, pre)) => (core, Some(pre)),
        None => (rest, None),
    };
    let [major, minor, patch] = parse_core(s, core)?;
    if let Some(pre) = pre {
        check_identifiers(core, pre, Part::Pre)?;
    }
    if let Some(build) = build {
        check_identifiers(core, build, Part::Build)?;
    }
    Ok((
        major,
        minor,
        patch,
        pre.unwrap_or_default().to_string(),
        build.unwrap_or_default().to_string(),
    ))
}

/// Parses `MAJOR.MINOR.PATCH`; `s` is the whole literal, for suggestions.
fn parse_core(s: &str, core: &str) -> Result<[u64; 3], String> {
    let parts: Vec<&str> = core.split('.').collect();
    if parts.len() > 3 {
        return Err(r#"too many components; SemVer has MAJOR.MINOR.PATCH, like "1.2.3""#.into());
    }
    let mut numbers = [0; 3];
    for (i, part) in parts.iter().enumerate() {
        let name = NAMES[i];
        if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
            return Err(format!("{name} version `{part}` is not a number"));
        }
        if part.len() > 1 && part.starts_with('0') {
            let mut fixed = parts.clone();
            fixed[i] = trim_zeros(part);
            let tail = &s[core.len()..];
            return Err(format!(
                r#"{name} version can't have leading zeros; write "{}{tail}""#,
                fixed.join(".")
            ));
        }
        numbers[i] = part
            .parse()
            .map_err(|_| format!("{name} version is too large for u64"))?;
    }
    match parts.len() {
        1 => Err(format!(
            r#"missing minor and patch versions; write e.g. "{}.0.0""#,
            numbers[0]
        )),
        2 => Err(format!(
            r#"missing patch version; write e.g. "{}.{}.0""#,
            numbers[0], numbers[1]
        )),
        _ => Ok(numbers),
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Part {
    Pre,
    Build,
}

/// Checks the dot-separated identifiers after `-` or `+`.
fn check_identifiers(core: &str, text: &str, part: Part) -> Result<(), String> {
    let name = match part {
        Part::Pre => "pre-release",
        Part::Build => "build metadata",
    };
    if text.is_empty() {
        return Err(match part {
            Part::Pre => {
                format!(r#"empty pre-release; remove the "-" or write e.g. "{core}-beta.1""#)
            }
            Part::Build => {
                format!(r#"empty build metadata; remove the "+" or write e.g. "{core}+build.5""#)
            }
        });
    }
    if let Some(c) = text
        .chars()
        .find(|&c| !(c.is_ascii_alphanumeric() || c == '-' || c == '.'))
    {
        return Err(format!(
            "invalid character `{c}` in {name}; use letters, digits, hyphens and dots"
        ));
    }
    for identifier in text.split('.') {
        if identifier.is_empty() {
            return Err(format!(r#"empty identifier in {name} "{text}""#));
        }
        let numeric = identifier.bytes().all(|b| b.is_ascii_digit());
        if part == Part::Pre && numeric && identifier.len() > 1 && identifier.starts_with('0') {
            return Err(format!(
                r#"numeric pre-release identifier "{identifier}" can't have leading zeros; write "{}""#,
                trim_zeros(identifier)
            ));
        }
    }
    Ok(())
}

/// `"007"` → `"7"`, `"00"` → `"0"`.
fn trim_zeros(digits: &str) -> &str {
    let trimmed = digits.trim_start_matches('0');
    if trimmed.is_empty() { "0" } else { trimmed }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(s: &str) -> (u64, u64, u64, String, String) {
        parse_version(s).unwrap_or_else(|e| panic!("{s:?}: {e}"))
    }

    fn err(s: &str) -> String {
        parse_version(s).expect_err(s)
    }

    #[test]
    fn version_values() {
        let v = |a, b, c, p: &str, m: &str| (a, b, c, p.to_string(), m.to_string());
        assert_eq!(ok("1.2.3"), v(1, 2, 3, "", ""));
        assert_eq!(ok("1.2.3-beta.1+build.5"), v(1, 2, 3, "beta.1", "build.5"));
        assert_eq!(ok("0.0.0"), v(0, 0, 0, "", ""));
        assert_eq!(ok("1.0.0+001"), v(1, 0, 0, "", "001"));
        assert_eq!(ok("1.0.0-x-y-z.--"), v(1, 0, 0, "x-y-z.--", ""));
        assert_eq!(ok("18446744073709551615.0.0"), v(u64::MAX, 0, 0, "", ""));
    }

    #[test]
    fn version_errors() {
        let cases = [
            (
                "",
                r#"empty version; write MAJOR.MINOR.PATCH, like "1.2.3""#,
            ),
            (" 1.2.3", "remove the spaces around the literal"),
            ("1.2.3 ", "remove the spaces around the literal"),
            ("v1.2.3", r#"remove the leading "v"; write "1.2.3""#),
            ("V1.2.3", r#"remove the leading "v"; write "1.2.3""#),
            (
                "1",
                r#"missing minor and patch versions; write e.g. "1.0.0""#,
            ),
            ("1.2", r#"missing patch version; write e.g. "1.2.0""#),
            (
                "1.2.3.4",
                r#"too many components; SemVer has MAJOR.MINOR.PATCH, like "1.2.3""#,
            ),
            ("1.x.3", "minor version `x` is not a number"),
            ("x.2.3", "major version `x` is not a number"),
            ("1.2.x", "patch version `x` is not a number"),
            ("1..3", "minor version `` is not a number"),
            (
                "01.2.3",
                r#"major version can't have leading zeros; write "1.2.3""#,
            ),
            (
                "1.002.3-rc.1",
                r#"minor version can't have leading zeros; write "1.2.3-rc.1""#,
            ),
            (
                "1.2.00",
                r#"patch version can't have leading zeros; write "1.2.0""#,
            ),
            (
                "99999999999999999999.0.0",
                "major version is too large for u64",
            ),
            (
                "1.2.3-",
                r#"empty pre-release; remove the "-" or write e.g. "1.2.3-beta.1""#,
            ),
            (
                "1.2.3+",
                r#"empty build metadata; remove the "+" or write e.g. "1.2.3+build.5""#,
            ),
            (
                "1.2.3-beta..1",
                r#"empty identifier in pre-release "beta..1""#,
            ),
            ("1.2.3+b..1", r#"empty identifier in build metadata "b..1""#),
            (
                "1.2.3-01",
                r#"numeric pre-release identifier "01" can't have leading zeros; write "1""#,
            ),
            (
                "1.2.3-beta_1",
                "invalid character `_` in pre-release; use letters, digits, hyphens and dots",
            ),
            (
                "1.2.3+b_1",
                "invalid character `_` in build metadata; use letters, digits, hyphens and dots",
            ),
        ];
        for (input, message) in cases {
            assert_eq!(err(input), message, "input {input:?}");
        }
    }

    fn splitmix(state: &mut u64) -> u64 {
        *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = *state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// A random well-formed SemVer string, biased towards small numbers and shared identifiers.
    fn well_formed(state: &mut u64) -> String {
        const WORDS: [&str; 6] = ["alpha", "beta", "rc", "x-1", "-", "a0"];
        let mut s = format!(
            "{}.{}.{}",
            splitmix(state) % 3,
            splitmix(state) % 3,
            splitmix(state) % 3
        );
        let identifiers = |state: &mut u64| {
            (0..1 + splitmix(state) % 3)
                .map(|_| match splitmix(state) % 4 {
                    0 => WORDS[(splitmix(state) % 6) as usize].to_string(),
                    1 => (splitmix(state) % 20).to_string(),
                    2 => splitmix(state).to_string(),
                    _ => format!("{}{}", splitmix(state), splitmix(state)),
                })
                .collect::<Vec<_>>()
                .join(".")
        };
        if splitmix(state).is_multiple_of(2) {
            s = format!("{s}-{}", identifiers(state));
        }
        if splitmix(state).is_multiple_of(3) {
            s = format!("{s}+{}", identifiers(state));
        }
        s
    }

    #[test]
    fn version_oracle() {
        let mut state = 11u64;
        let alphabet = b"0123456789.-+abv";
        for i in 0..40_000 {
            let text = if i % 2 == 0 {
                well_formed(&mut state)
            } else {
                (0..splitmix(&mut state) % 15)
                    .map(|_| alphabet[(splitmix(&mut state) % 16) as usize] as char)
                    .collect()
            };
            let ours = parse_version(&text).ok();
            let theirs = semver::Version::parse(&text).ok().map(|v| {
                (
                    v.major,
                    v.minor,
                    v.patch,
                    v.pre.to_string(),
                    v.build.to_string(),
                )
            });
            assert_eq!(ours, theirs, "input {text:?}");
            if i % 2 == 0 {
                assert!(ours.is_some(), "generator made an invalid version {text:?}");
            }
        }
    }
}
