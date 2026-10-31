//! `mime!`: media types such as `"application/json"` or `"text/html; charset=utf-8"`.

/// RFC 9110 `tchar`: the characters of a token.
pub fn is_tchar(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b)
}

const EXAMPLE: &str = r#"write type/subtype, like "application/json""#;
const WILDCARD: &str = "`*` wildcards are media ranges for Accept headers, not media types";

/// A parsed media type: `(essence, slash, params)`.
pub type Parsed = (String, usize, Vec<(String, String)>);

/// Parses a media type into `(essence, slash, params)`: the lowercased `type/subtype`,
/// the index of its `/`, and the parameters with lowercased names and unquoted values.
pub fn parse_media_type(s: &str) -> Result<Parsed, String> {
    if s.is_empty() {
        return Err(format!("empty media type; {EXAMPLE}"));
    }
    if s.starts_with(char::is_whitespace) || s.ends_with(char::is_whitespace) {
        return Err("remove the spaces around the literal".into());
    }
    let (head, mut rest) = match s.split_once(';') {
        Some((head, rest)) => (head.trim_end_matches(is_ows), Some(rest)),
        None => (s, None),
    };
    let Some((kind, subtype)) = head.split_once('/') else {
        return Err(format!("missing `/`; {EXAMPLE}"));
    };
    if kind == "*" || subtype == "*" {
        return Err(WILDCARD.into());
    }
    if kind.is_empty() {
        return Err(r#"empty type; write e.g. "text/plain""#.into());
    }
    if subtype.is_empty() {
        return Err(r#"empty subtype; write e.g. "text/plain""#.into());
    }
    check_restricted_name(kind, "type")?;
    check_restricted_name(subtype, "subtype")?;
    let essence = head.to_ascii_lowercase();
    let mut params: Vec<(String, String)> = Vec::new();
    while let Some(text) = rest {
        let text = text.trim_start_matches(is_ows);
        if text.is_empty() {
            return Err("remove the trailing `;`".into());
        }
        let (name, value, after) = parse_param(text)?;
        if params.iter().any(|(seen, _)| *seen == name) {
            return Err(format!("parameter `{name}` appears twice"));
        }
        let value = if name == "charset" {
            value.to_ascii_lowercase()
        } else {
            value
        };
        params.push((name, value));
        rest = after;
    }
    Ok((essence, kind.len(), params))
}

fn is_ows(c: char) -> bool {
    c == ' ' || c == '\t'
}

/// RFC 6838 `restricted-name`: 1-127 characters, starting with a letter or digit.
fn check_restricted_name(name: &str, what: &str) -> Result<(), String> {
    let allowed = |c: char| c.is_ascii_alphanumeric() || "!#$&-^_.+".contains(c);
    if let Some(c) = name.chars().find(|&c| !allowed(c)) {
        return Err(format!("invalid character `{c}` in {what} `{name}`"));
    }
    if !name.starts_with(|c: char| c.is_ascii_alphanumeric()) {
        return Err(format!("{what} `{name}` must start with a letter or digit"));
    }
    if name.len() > 127 {
        return Err(format!("{what} is longer than 127 characters"));
    }
    Ok(())
}

/// Parses `name=value` at the start of `text` (OWS already skipped). Returns the
/// lowercased name, the unquoted value, and what follows the next `;`, if any.
fn parse_param(text: &str) -> Result<(String, String, Option<&str>), String> {
    let end = text.find(['=', ';']).unwrap_or(text.len());
    let written = &text[..end];
    let name = written.trim_end_matches(is_ows);
    if name.is_empty() {
        return Err("missing parameter name before `=`".into());
    }
    if let Some(c) = name.chars().find(|&c| !c.is_ascii() || !is_tchar(c as u8)) {
        return Err(format!(
            "invalid character `{c}` in parameter name `{name}`"
        ));
    }
    let lower = name.to_ascii_lowercase();
    if !text[end..].starts_with('=') {
        let example = if lower == "charset" { "utf-8" } else { "x" };
        return Err(format!(
            "parameter `{name}` needs a value, like {name}={example}"
        ));
    }
    let value_text = &text[end + 1..];
    if written.len() != name.len() || value_text.starts_with(is_ows) {
        return Err(format!("no spaces around `=` in parameter `{name}`"));
    }
    let (value, after) = if let Some(quoted) = value_text.strip_prefix('"') {
        let mut value = String::new();
        let mut chars = quoted.char_indices();
        let close = loop {
            match chars.next() {
                None => return Err(format!("unterminated quoted value in parameter `{name}`")),
                Some((i, '"')) => break i,
                Some((_, '\\')) => match chars.next() {
                    Some((_, c)) => value.push(c),
                    None => {
                        return Err(format!("unterminated quoted value in parameter `{name}`"));
                    }
                },
                Some((_, c)) if c.is_control() && c != '\t' => {
                    return Err(format!(
                        "invalid character {c:?} in the value of parameter `{name}`"
                    ));
                }
                Some((_, c)) => value.push(c),
            }
        };
        let after = quoted[close + 1..].trim_start_matches(is_ows);
        match after.strip_prefix(';') {
            Some(next) => (value, Some(next)),
            None if after.is_empty() => (value, None),
            None => {
                return Err(format!(
                    "unexpected text after the quoted value of parameter `{name}`; put a `;` before the next parameter"
                ));
            }
        }
    } else {
        let (token, after) = match value_text.split_once(';') {
            Some((token, next)) => (token.trim_end_matches(is_ows), Some(next)),
            None => (value_text, None),
        };
        if token.is_empty() {
            return Err(format!("parameter `{name}` has an empty value"));
        }
        if let Some(c) = token.chars().find(|&c| !c.is_ascii() || !is_tchar(c as u8)) {
            return Err(format!(
                "invalid character `{c}` in the value of parameter `{name}`; put the value in quotes"
            ));
        }
        (token.to_string(), after)
    };
    Ok((lower, value, after))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(s: &str) -> Parsed {
        parse_media_type(s).unwrap_or_else(|e| panic!("{s:?}: {e}"))
    }

    fn err(s: &str) -> String {
        parse_media_type(s).expect_err(s)
    }

    fn params(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|&(n, v)| (n.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn mime_values() {
        assert_eq!(
            ok("application/json"),
            ("application/json".into(), 11, vec![])
        );
        assert_eq!(
            ok("Text/HTML; Charset=UTF-8"),
            ("text/html".into(), 4, params(&[("charset", "utf-8")]))
        );
        assert_eq!(
            ok(r#"multipart/form-data; boundary="a b\"c""#),
            (
                "multipart/form-data".into(),
                9,
                params(&[("boundary", "a b\"c")])
            )
        );
        assert_eq!(ok("application/vnd.api+json").0, "application/vnd.api+json");
        assert_eq!(
            ok("text/plain;format=flowed"),
            ("text/plain".into(), 4, params(&[("format", "flowed")]))
        );
        assert_eq!(
            ok("text/plain \t;\tformat=flowed ; a=\"\""),
            (
                "text/plain".into(),
                4,
                params(&[("format", "flowed"), ("a", "")])
            )
        );
    }

    #[test]
    fn mime_errors() {
        let cases = [
            (
                "",
                r#"empty media type; write type/subtype, like "application/json""#,
            ),
            (" text/plain", "remove the spaces around the literal"),
            (
                "json",
                r#"missing `/`; write type/subtype, like "application/json""#,
            ),
            ("text/", r#"empty subtype; write e.g. "text/plain""#),
            ("/plain", r#"empty type; write e.g. "text/plain""#),
            (
                "text/*",
                "`*` wildcards are media ranges for Accept headers, not media types",
            ),
            (
                "*/*",
                "`*` wildcards are media ranges for Accept headers, not media types",
            ),
            ("te xt/plain", "invalid character ` ` in type `te xt`"),
            ("text/pl@in", "invalid character `@` in subtype `pl@in`"),
            (
                "text/plain; ch@rset=x",
                "invalid character `@` in parameter name `ch@rset`",
            ),
            ("text/plain;", "remove the trailing `;`"),
            ("text/plain; a=1;", "remove the trailing `;`"),
            (
                "text/plain; charset",
                "parameter `charset` needs a value, like charset=utf-8",
            ),
            (
                "text/plain; charset=",
                "parameter `charset` has an empty value",
            ),
            (
                "text/plain; charset = utf-8",
                "no spaces around `=` in parameter `charset`",
            ),
            (
                "text/plain; charset= utf-8",
                "no spaces around `=` in parameter `charset`",
            ),
            ("text/plain; a=1; A=2", "parameter `a` appears twice"),
            (
                "text/plain; a=\"x",
                "unterminated quoted value in parameter `a`",
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

    fn pick(state: &mut u64, chars: &[u8], len: u64) -> String {
        (0..len)
            .map(|_| chars[(splitmix(state) % chars.len() as u64) as usize] as char)
            .collect()
    }

    /// Our normalized form, written the way `constlit::MediaType`'s `Display` writes it.
    fn normalized((essence, _, params): &Parsed) -> String {
        let mut out = essence.clone();
        for (name, value) in params {
            let token = !value.is_empty() && value.bytes().all(is_tchar);
            if token {
                out += &format!("; {name}={value}");
            } else {
                let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
                out += &format!("; {name}=\"{escaped}\"");
            }
        }
        out
    }

    #[test]
    fn mime_oracle() {
        const ALNUM: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
        const NAME_CHARS: &[u8] = b"abcXYZ019!#$&-^_.+";
        const TCHARS: &[u8] = b"abcXYZ019!#$%&'*+-.^_`|~";
        // The `mime` crate rejects `""` and `\"`, both valid in RFC 9110; `to_mime` reports those.
        const QUOTED: &[u8] = b"abc XYZ;=,/()";
        let mut state = 5u64;
        for i in 0..5_000 {
            let name = |state: &mut u64| {
                let len = splitmix(state) % 12;
                format!("{}{}", pick(state, ALNUM, 1), pick(state, NAME_CHARS, len))
            };
            let mut text = format!("{}/{}", name(&mut state), name(&mut state));
            let mut seen = Vec::new();
            for _ in 0..splitmix(&mut state) % 4 {
                let len = 1 + splitmix(&mut state) % 8;
                let param = if i % 7 == 0 {
                    "charset".to_string()
                } else {
                    pick(&mut state, TCHARS, len)
                };
                if seen.contains(&param.to_ascii_lowercase()) {
                    continue;
                }
                seen.push(param.to_ascii_lowercase());
                let len = splitmix(&mut state) % 8;
                let value = if splitmix(&mut state).is_multiple_of(2) {
                    pick(&mut state, TCHARS, 1 + len)
                } else {
                    format!("\"{}\"", pick(&mut state, QUOTED, 1 + len))
                };
                text += &format!("; {param}={value}");
            }
            let ours = ok(&text);
            let out = normalized(&ours);
            let theirs: mime::Mime = out
                .parse()
                .unwrap_or_else(|e| panic!("{text:?} -> {out:?}: {e}"));
            assert_eq!(theirs.essence_str(), ours.0, "{text:?}");
            let their_params: Vec<(String, String)> = theirs
                .params()
                .map(|(n, v)| (n.as_str().to_ascii_lowercase(), v.as_str().to_string()))
                .collect();
            assert_eq!(their_params.len(), ours.2.len(), "{text:?}");
            for ((n1, v1), (n2, v2)) in their_params.iter().zip(&ours.2) {
                assert_eq!(n1, n2, "{text:?}");
                if n1 == "charset" {
                    assert!(v1.eq_ignore_ascii_case(v2), "{text:?}");
                } else {
                    assert_eq!(v1, v2, "{text:?}");
                }
            }
        }
    }
}
