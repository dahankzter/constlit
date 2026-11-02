//! `version_req!`: Cargo version requirements, parsed like the `semver` crate.
//!
//! Every requirement `semver::VersionReq::parse` accepts is accepted, with the
//! same comparators, apart from build metadata (which semver silently ignores)
//! and surrounding whitespace. Suggestions are offered only when they parse.

use crate::version::{Part, check_identifiers};

/// One comparator: the operator (`"="`, `">"`, `">="`, `"<"`, `"<="`, `"~"`,
/// `"^"` or `"*"` for a wildcard), the version parts given, and the pre-release.
pub struct Comparator {
    pub op: &'static str,
    pub major: u64,
    pub minor: Option<u64>,
    pub patch: Option<u64>,
    pub pre: String,
}

const OPS: [&str; 7] = [">=", "<=", ">", "<", "=", "~", "^"];

/// Parses a requirement into its comparators; `"*"` has none.
pub fn parse_version_req(s: &str) -> Result<Vec<Comparator>, String> {
    if s.is_empty() {
        return Err(r#"empty version requirement; write e.g. "^1.2" or ">=1.2, <2""#.into());
    }
    if s.starts_with(char::is_whitespace) || s.ends_with(char::is_whitespace) {
        return Err("remove the spaces around the literal".into());
    }
    // semver skips plain spaces only; a tab or non-breaking space is an error there.
    if s.chars().any(|c| c.is_whitespace() && c != ' ') {
        let plain: String = s
            .chars()
            .map(|c| if c.is_whitespace() { ' ' } else { c })
            .collect();
        return suggest("use plain spaces, not tabs or other whitespace", &plain);
    }
    if s.contains("||") {
        return Err(r#"npm's "||" isn't Cargo syntax; a requirement matches all its comparators, so write one range"#.into());
    }
    if let Some((low, high)) = s.split_once(" - ") {
        return suggest(
            "npm's hyphen ranges aren't Cargo syntax",
            &format!(">={}, <={}", low.trim_matches(' '), high.trim_matches(' ')),
        );
    }
    if matches!(s, "*" | "x" | "X") {
        return Ok(Vec::new());
    }
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() > 1 && parts[parts.len() - 1].trim_matches(' ').is_empty() {
        return Err(r#"remove the trailing ",""#.into());
    }
    if parts.len() > 32 {
        return Err("a requirement has at most 32 comparators".into());
    }
    let mut comparators = Vec::new();
    for (i, part) in parts.iter().enumerate() {
        let text = part.trim_matches(' ');
        if text.is_empty() {
            return Err(if i + 1 == parts.len() {
                r#"remove the trailing ",""#.into()
            } else {
                "empty comparator between commas".into()
            });
        }
        if matches!(text, "*" | "x" | "X") {
            return Err(r#""*" matches every version, so it must be the only comparator"#.into());
        }
        comparators.push(comparator(s, text)?);
    }
    Ok(comparators)
}

/// The byte offset of `part` (a subslice of `whole`) in `whole`.
fn offset(whole: &str, part: &str) -> usize {
    part.as_ptr() as usize - whole.as_ptr() as usize
}

/// `Err("<message>: \"<fixed>\"")` when `fixed` parses; otherwise its own error.
fn suggest<T>(message: &str, fixed: &str) -> Result<T, String> {
    parse_version_req(fixed)?;
    Err(format!(r#"{message}: "{fixed}""#))
}

/// `whole` with the bytes `start..end` replaced by `with`.
fn splice(whole: &str, start: usize, end: usize, with: &str) -> String {
    format!("{}{with}{}", &whole[..start], &whole[end..])
}

fn comparator(whole: &str, text: &str) -> Result<Comparator, String> {
    if text.starts_with("=>") {
        let at = offset(whole, text);
        return suggest(
            r#"write ">=" instead of "=>""#,
            &splice(whole, at, at + 2, ">="),
        );
    }
    if let Some(after) = text.strip_prefix("~>") {
        // Ruby's "~> 1.2" is ">=1.2, <2" (Cargo's ^1.2); "~> 1" and "~> 1.2.3" are
        // Cargo's "~1" and "~1.2.3".
        let after = after.trim_start_matches(' ');
        let version = after.split(' ').next().unwrap_or(after);
        let core = version.split(['-', '+']).next().unwrap_or(version);
        let parts: Vec<&str> = core.split('.').collect();
        let cargo = match parts.as_slice() {
            ["0", _] => format!(">={core}, <1"),
            [_, _] => format!("^{core}"),
            _ => format!("~{version}"),
        };
        let at = offset(whole, text);
        let end = offset(whole, version) + version.len();
        return suggest(
            r#""~>" is Ruby's operator; in Cargo write"#,
            &splice(whole, at, end, &cargo),
        );
    }
    let written = OPS
        .iter()
        .find(|op| text.starts_with(**op))
        .copied()
        .unwrap_or("");
    let rest = text[written.len()..].trim_start_matches(' ');
    match rest.chars().next() {
        Some(c) if c.is_ascii_digit() || matches!(c, '*' | 'x' | 'X' | 'v' | 'V') => {}
        Some(c) if written.is_empty() && c.is_ascii_punctuation() && c != '.' => {
            return Err(format!(
                r#"unknown operator "{c}"; use =, >, >=, <, <=, ~ or ^"#
            ));
        }
        _ => return Err(r#"expected a version, like "1.2" or "^1.2.3""#.into()),
    }
    if rest.starts_with(['v', 'V']) {
        let at = offset(whole, rest);
        return suggest(r#"remove the "v""#, &splice(whole, at, at + 1, ""));
    }
    // A second comparator without a comma: ">=1.2 <2".
    let version = rest.split(' ').next().unwrap_or(rest);
    if version.len() < rest.len() {
        let at = offset(whole, rest) + version.len();
        return suggest(
            r#"separate comparators with ",""#,
            &splice(whole, at, at, ","),
        );
    }
    let (version, build) = match version.split_once('+') {
        Some((version, build)) => (version, Some(build)),
        None => (version, None),
    };
    if let Some(build) = build {
        let at = offset(whole, version) + version.len();
        return suggest(
            "build metadata is ignored in a requirement; remove it",
            &splice(whole, at, at + 1 + build.len(), ""),
        );
    }
    let (core, pre) = match version.split_once('-') {
        Some((core, pre)) => (core, Some(pre)),
        None => (version, None),
    };
    let pieces: Vec<&str> = core.split('.').collect();
    if pieces.len() > 3 {
        return Err("a version has at most three numbers, MAJOR.MINOR.PATCH".into());
    }
    let names = ["major", "minor", "patch"];
    let mut numbers: [Option<u64>; 3] = [None; 3];
    let mut wildcard = false;
    for (i, piece) in pieces.iter().enumerate() {
        // Only more wildcards may follow a wildcard ("1.*.*").
        // The suggestion keeps the operator and the other comparators.
        let rest_end = offset(whole, rest) + rest.len();
        if wildcard && !matches!(*piece, "*" | "x" | "X") {
            let wildcard_end = offset(whole, pieces[i - 1]) + pieces[i - 1].len();
            return suggest(
                "nothing can follow a wildcard",
                &splice(whole, wildcard_end, rest_end, ""),
            );
        }
        if matches!(*piece, "*" | "x" | "X") {
            if i == 0 {
                if !written.is_empty() {
                    return Err(
                        r#"a wildcard can't follow an operator; write "*" for any version"#.into(),
                    );
                }
                let at = offset(whole, rest);
                return suggest(
                    "nothing can follow a wildcard",
                    &splice(whole, at, rest_end, "*"),
                );
            }
            wildcard = true;
            continue;
        }
        if piece.is_empty() {
            return Err(if i == 0 {
                r#"expected a version, like "1.2" or "^1.2.3""#.into()
            } else {
                format!(r#"missing {} version after ".""#, names[i])
            });
        }
        if !piece.bytes().all(|b| b.is_ascii_digit()) {
            return Err(if i == 0 {
                r#"expected a version, like "1.2" or "^1.2.3""#.into()
            } else {
                format!("{} version `{piece}` is not a number", names[i])
            });
        }
        if piece.len() > 1 && piece.starts_with('0') {
            let at = offset(whole, piece);
            let trimmed = piece.trim_start_matches('0');
            let trimmed = if trimmed.is_empty() { "0" } else { trimmed };
            return suggest(
                "version numbers can't have leading zeros",
                &splice(whole, at, at + piece.len(), trimmed),
            );
        }
        numbers[i] = Some(
            piece
                .parse()
                .map_err(|_| format!("{} version is too large for u64", names[i]))?,
        );
    }
    let [major, minor, patch] = numbers;
    let pre = match pre {
        Some(pre) => {
            if minor.is_none() || patch.is_none() {
                // Offer the example only when it is valid; otherwise report its problem.
                let example = format!(
                    "{written}{}.{}.0-{pre}",
                    major.unwrap_or(0),
                    minor.unwrap_or(0)
                );
                parse_version_req(&example)?;
                return Err(format!(
                    r#"a pre-release needs MAJOR.MINOR.PATCH, like "{example}""#
                ));
            }
            check_identifiers(core, pre, Part::Pre)?;
            pre.to_string()
        }
        None => String::new(),
    };
    let op = match (written, wildcard) {
        ("", true) => "*",
        ("", false) => "^",
        (op, _) => op,
    };
    Ok(Comparator {
        op,
        major: major.expect("checked above"),
        minor,
        patch,
        pre,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    type Summary = (&'static str, u64, Option<u64>, Option<u64>, String);

    fn summary(s: &str) -> Vec<Summary> {
        parse_version_req(s)
            .unwrap_or_else(|e| panic!("{s:?}: {e}"))
            .into_iter()
            .map(|c| (c.op, c.major, c.minor, c.patch, c.pre))
            .collect()
    }

    #[test]
    fn version_req_values() {
        let c = |op, major, minor, patch, pre: &str| (op, major, minor, patch, pre.to_string());
        assert_eq!(summary("1.2"), [c("^", 1, Some(2), None, "")]);
        assert_eq!(summary("^1.2.3"), [c("^", 1, Some(2), Some(3), "")]);
        assert_eq!(summary("~1.2"), [c("~", 1, Some(2), None, "")]);
        assert_eq!(
            summary("=1.2.3-alpha.1"),
            [c("=", 1, Some(2), Some(3), "alpha.1")]
        );
        let range = [c(">=", 1, Some(2), None, ""), c("<", 2, None, None, "")];
        assert_eq!(summary(">=1.2, <2"), range);
        assert_eq!(summary(">= 1.2 , < 2"), range);
        assert_eq!(summary("1.*"), [c("*", 1, None, None, "")]);
        assert_eq!(summary("1.2.x"), [c("*", 1, Some(2), None, "")]);
        assert_eq!(summary(">1.*"), [c(">", 1, None, None, "")]);
        assert!(summary("*").is_empty());
        assert!(summary("X").is_empty());
        assert_eq!(
            summary("<1.2.3-rc.1"),
            [c("<", 1, Some(2), Some(3), "rc.1")]
        );
    }

    #[test]
    fn version_req_errors() {
        let many = vec!["1"; 33].join(",");
        let many_trailing = format!("{},", vec!["1"; 32].join(","));
        let cases = [
            (
                "",
                r#"empty version requirement; write e.g. "^1.2" or ">=1.2, <2""#,
            ),
            (" ^1", "remove the spaces around the literal"),
            (">=1.2 <2", r#"separate comparators with ",": ">=1.2, <2""#),
            (
                "^1.2.3.4",
                r#"a version has at most three numbers, MAJOR.MINOR.PATCH"#,
            ),
            (
                "1.2.3-",
                r#"empty pre-release; remove the "-" or write e.g. "1.2.3-beta.1""#,
            ),
            (">=v1.2", r#"remove the "v": ">=1.2""#),
            (
                "^01.2",
                r#"version numbers can't have leading zeros: "^1.2""#,
            ),
            (
                "=*",
                r#"a wildcard can't follow an operator; write "*" for any version"#,
            ),
            (
                "1.2.3+build",
                r#"build metadata is ignored in a requirement; remove it: "1.2.3""#,
            ),
            ("1.*.3", r#"nothing can follow a wildcard: "1.*""#),
            (
                ">=1.*.3, <2",
                r#"nothing can follow a wildcard: ">=1.*, <2""#,
            ),
            ("1,", r#"remove the trailing ",""#),
            (", 1", "empty comparator between commas"),
            ("1,,2", "empty comparator between commas"),
            (
                "*, 1",
                r#""*" matches every version, so it must be the only comparator"#,
            ),
            ("=>1", r#"write ">=" instead of "=>": ">=1""#),
            ("~>1", r#""~>" is Ruby's operator; in Cargo write: "~1""#),
            (
                "~>1.2",
                r#""~>" is Ruby's operator; in Cargo write: "^1.2""#,
            ),
            (
                "~>0.2",
                r#""~>" is Ruby's operator; in Cargo write: ">=0.2, <1""#,
            ),
            (
                "~>1.2.3",
                r#""~>" is Ruby's operator; in Cargo write: "~1.2.3""#,
            ),
            (
                ">=1.2,\t<2",
                r#"use plain spaces, not tabs or other whitespace: ">=1.2, <2""#,
            ),
            (
                "1\u{a0},2",
                r#"use plain spaces, not tabs or other whitespace: "1 ,2""#,
            ),
            (
                "1.2-",
                r#"empty pre-release; remove the "-" or write e.g. "1.2.0-beta.1""#,
            ),
            (
                "1.2.3 - 2.0.0",
                r#"npm's hyphen ranges aren't Cargo syntax: ">=1.2.3, <=2.0.0""#,
            ),
            (
                "^1.2 || ^2",
                r#"npm's "||" isn't Cargo syntax; a requirement matches all its comparators, so write one range"#,
            ),
            ("!1", r#"unknown operator "!"; use =, >, >=, <, <=, ~ or ^"#),
            ("1.", r#"missing minor version after ".""#),
            (".1", r#"expected a version, like "1.2" or "^1.2.3""#),
            ("a", r#"expected a version, like "1.2" or "^1.2.3""#),
            ("18446744073709551616", "major version is too large for u64"),
            (
                "^1.2-alpha",
                r#"a pre-release needs MAJOR.MINOR.PATCH, like "^1.2.0-alpha""#,
            ),
            (
                "1.2.3-alpha.01",
                r#"numeric pre-release identifier "01" can't have leading zeros; write "1""#,
            ),
            (&many, "a requirement has at most 32 comparators"),
            (&many_trailing, r#"remove the trailing ",""#),
        ];
        for (input, message) in cases {
            assert_eq!(
                parse_version_req(input).err().as_deref(),
                Some(message),
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

    fn pick<'a>(state: &mut u64, items: &[&'a str]) -> &'a str {
        items[(splitmix(state) % items.len() as u64) as usize]
    }

    /// The literal a message suggests: the quoted text after its last `: "`,
    /// `write "` or `like "`.
    fn suggestion(message: &str) -> Option<&str> {
        let start = [": \"", "write \"", "like \""]
            .iter()
            .filter_map(|m| message.rfind(m).map(|i| i + m.len()))
            .max()?;
        let end = message[start..].find('"')?;
        Some(&message[start..start + end])
    }

    #[test]
    fn version_req_oracle() {
        // Mostly valid, so accepted requirements are well covered too.
        let ops = [
            "", "", "", "", "=", ">", ">=", "<", "<=", "~", "^", "^", "=>", "!",
        ];
        let nums = [
            "0", "1", "2", "10", "3", "1", "2", "0", "*", "x", "X", "01", "a",
        ];
        let tails = [
            "", "", "", "", "", "", "", "-alpha.1", "-beta", "+build", "-01",
        ];
        let joins = [",", ", ", ", ", ", ", " ,", " ", ",\t", "\u{a0},"];
        let mut state = 8u64;
        let (mut accepted, mut compared) = (0, 0);
        for _ in 0..50_000 {
            let mut s = String::new();
            for i in 0..1 + splitmix(&mut state) % 3 {
                if i > 0 {
                    s.push_str(pick(&mut state, &joins));
                }
                let op = pick(&mut state, &ops);
                s.push_str(op);
                if !op.is_empty() && splitmix(&mut state).is_multiple_of(5) {
                    s.push(' ');
                }
                let parts: Vec<&str> = (0..1 + splitmix(&mut state) % 3)
                    .map(|_| pick(&mut state, &nums))
                    .collect();
                s.push_str(&parts.join("."));
                s.push_str(pick(&mut state, &tails));
            }
            compared += 1;
            let ours = parse_version_req(&s);
            if let Err(message) = &ours
                && let Some(fixed) = suggestion(message)
            {
                assert!(
                    parse_version_req(fixed).is_ok(),
                    "{s:?}: suggestion {fixed:?} is invalid"
                );
            }
            match (semver::VersionReq::parse(&s), ours) {
                (Ok(theirs), Ok(ours)) => {
                    accepted += 1;
                    let theirs: Vec<_> = theirs
                        .comparators
                        .iter()
                        .map(|c| {
                            let op = match c.op {
                                semver::Op::Exact => "=",
                                semver::Op::Greater => ">",
                                semver::Op::GreaterEq => ">=",
                                semver::Op::Less => "<",
                                semver::Op::LessEq => "<=",
                                semver::Op::Tilde => "~",
                                semver::Op::Caret => "^",
                                semver::Op::Wildcard => "*",
                                _ => "?",
                            };
                            (op, c.major, c.minor, c.patch, c.pre.to_string())
                        })
                        .collect();
                    let ours: Vec<_> = ours
                        .into_iter()
                        .map(|c| (c.op, c.major, c.minor, c.patch, c.pre))
                        .collect();
                    assert_eq!(ours, theirs, "input {s:?}");
                }
                (Err(_), Err(_)) => {}
                (Ok(_), Err(message)) => assert!(
                    (message.starts_with("build metadata is ignored") && s.contains('+'))
                        || (message == "remove the spaces around the literal" && s.trim() != s)
                        || (message.starts_with("use plain spaces")
                            && s.chars().any(|c| c.is_whitespace() && c != ' ')),
                    "semver accepts {s:?}, we say: {message}"
                ),
                (Err(e), Ok(_)) => panic!("semver rejects {s:?} ({e}), we accept it"),
            }
        }
        assert!(
            compared == 50_000 && accepted >= 5000,
            "only {accepted} accepted"
        );
    }
}
