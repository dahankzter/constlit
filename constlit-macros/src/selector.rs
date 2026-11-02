//! `k8s::selector!`: Kubernetes label selectors, as `labels.Parse` reads them.
//!
//! The lexer and grammar follow `k8s.io/apimachinery/pkg/labels/selector.go`;
//! the fixture in `testdata/k8s_selector.tsv` pins them. On top of Kubernetes,
//! an empty selector and empty values in a set are errors.

use crate::k8s_names::{check_label_key, check_label_value};

#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    Ident,
    Comma,
    Open,
    Close,
    Not,
    Eq,
    DoubleEq,
    NotEq,
    Gt,
    Lt,
    End,
}

#[derive(Clone, Copy, Debug)]
struct Token<'a> {
    kind: Kind,
    text: &'a str,
    start: usize,
    end: usize,
}

fn is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\r' | b'\n')
}

fn is_special(b: u8) -> bool {
    matches!(b, b'=' | b'!' | b'(' | b')' | b',' | b'>' | b'<')
}

/// Splits `s` into tokens like Kubernetes' lexer: runs of non-special,
/// non-space bytes are identifiers; `!=` and `==` are the only two-byte symbols.
fn lex(s: &str) -> Vec<Token<'_>> {
    let bytes = s.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    loop {
        while i < bytes.len() && is_space(bytes[i]) {
            i += 1;
        }
        let start = i;
        if i == bytes.len() {
            tokens.push(Token {
                kind: Kind::End,
                text: "",
                start,
                end: i,
            });
            return tokens;
        }
        let kind = if is_special(bytes[i]) {
            // Bytes, not a `&str` slice: the next character may be multi-byte.
            let (kind, len) = match (bytes[i], bytes.get(i + 1)) {
                (b'!', Some(b'=')) => (Kind::NotEq, 2),
                (b'=', Some(b'=')) => (Kind::DoubleEq, 2),
                (b'=', _) => (Kind::Eq, 1),
                (b'!', _) => (Kind::Not, 1),
                (b'(', _) => (Kind::Open, 1),
                (b')', _) => (Kind::Close, 1),
                (b',', _) => (Kind::Comma, 1),
                (b'>', _) => (Kind::Gt, 1),
                _ => (Kind::Lt, 1),
            };
            i += len;
            kind
        } else {
            while i < bytes.len() && !is_special(bytes[i]) && !is_space(bytes[i]) {
                i += 1;
            }
            Kind::Ident
        };
        tokens.push(Token {
            kind,
            text: &s[start..i],
            start,
            end: i,
        });
    }
}

/// `k8s::selector!`: checks a label selector.
pub fn check_selector(s: &str) -> Result<(), String> {
    if s.is_empty() {
        return Err("an empty selector matches everything; use your API's match-all form instead, e.g. LabelSelector::default()".into());
    }
    if s.starts_with(char::is_whitespace) || s.ends_with(char::is_whitespace) {
        return Err("remove the spaces around the literal".into());
    }
    let tokens = lex(s);
    let mut p = 0;
    loop {
        let first = tokens[p];
        match first.kind {
            Kind::Ident | Kind::Not => p = requirement(s, &tokens, p)?,
            Kind::Comma => return Err("empty requirement between commas".into()),
            Kind::Eq | Kind::DoubleEq | Kind::NotEq => {
                return Err(format!(r#"missing key before "{}""#, first.text));
            }
            _ => return Err(format!(r#"expected a key, found "{}""#, first.text)),
        }
        let next = tokens[p];
        match next.kind {
            Kind::End => return Ok(()),
            Kind::Comma => {
                p += 1;
                if tokens[p].kind == Kind::End {
                    return Err(r#"remove the trailing ",""#.into());
                }
            }
            _ => {
                let written = &s[first.start..tokens[p - 1].end];
                return Err(format!(
                    r#"expected "," or the end after "{written}", found "{}""#,
                    next.text
                ));
            }
        }
    }
}

/// Parses one requirement starting at `tokens[p]`; returns the index after it.
fn requirement(s: &str, tokens: &[Token<'_>], mut p: usize) -> Result<usize, String> {
    let negated = tokens[p].kind == Kind::Not;
    if negated {
        p += 1;
    }
    let key = tokens[p];
    if key.kind != Kind::Ident {
        return Err(if negated {
            r#"expected a key after "!""#.into()
        } else {
            format!(r#"expected a key, found "{}""#, key.text)
        });
    }
    check_label_key(key.text).map_err(|e| format!(r#"key "{}": {e}"#, key.text))?;
    p += 1;
    let op = tokens[p];
    if matches!(op.kind, Kind::End | Kind::Comma) {
        return Ok(p);
    }
    if negated {
        if matches!(op.kind, Kind::Eq | Kind::DoubleEq | Kind::NotEq) {
            let value = match tokens[p + 1] {
                t if t.kind == Kind::Ident => t.text,
                _ => "value",
            };
            return Err(format!(
                r#""!" checks that a key is absent; write "!{0}" or "{0}!={value}""#,
                key.text
            ));
        }
        return Ok(p);
    }
    p += 1;
    match op.kind {
        Kind::Eq | Kind::DoubleEq | Kind::NotEq => {
            let value = tokens[p];
            match value.kind {
                Kind::End | Kind::Comma => Ok(p),
                Kind::Ident => {
                    check_label_value(value.text)
                        .map_err(|e| format!(r#"value "{}": {e}"#, value.text))?;
                    Ok(p + 1)
                }
                _ => Err(format!(
                    r#"expected a value after "{}", found "{}""#,
                    op.text, value.text
                )),
            }
        }
        Kind::Gt | Kind::Lt => {
            let value = tokens[p];
            if value.kind != Kind::Ident {
                return Err(format!(r#"expected a whole number after "{}""#, op.text));
            }
            // Kubernetes parses an int64 that must also be a valid label value,
            // which leaves plain digits.
            let digits = value.text.bytes().all(|b| b.is_ascii_digit());
            if !digits || value.text.parse::<i64>().is_err() {
                return Err(format!(
                    r#""{}" after "{}" must be a whole number"#,
                    value.text, op.text
                ));
            }
            Ok(p + 1)
        }
        Kind::Ident if op.text == "in" || op.text == "notin" => set(s, tokens, p, op.text),
        _ => Err(format!(
            r#"expected an operator or "," after "{}", found "{}""#,
            key.text, op.text
        )),
    }
}

/// Parses `( value, ... )` at `tokens[p]` after `in` or `notin`.
fn set(s: &str, tokens: &[Token<'_>], mut p: usize, op: &str) -> Result<usize, String> {
    let open = tokens[p];
    if open.kind != Kind::Open {
        return Err(format!(r#"expected "(" after "{op}""#));
    }
    p += 1;
    if tokens[p].kind == Kind::Close {
        return Err(format!(r#""{op}" needs at least one value"#));
    }
    let mut previous = open;
    let mut empty = false;
    loop {
        let token = tokens[p];
        match token.kind {
            Kind::Ident => {
                if previous.kind == Kind::Ident {
                    return Err(format!(
                        r#"expected "," or ")" after "{}", found "{}""#,
                        previous.text, token.text
                    ));
                }
                check_label_value(token.text)
                    .map_err(|e| format!(r#"value "{}": {e}"#, token.text))?;
            }
            Kind::Comma => empty |= previous.kind != Kind::Ident,
            Kind::Close => {
                if empty || previous.kind == Kind::Comma {
                    let written = &s[open.start..token.end];
                    return Err(format!(
                        r#"empty value in "{written}"; remove the extra ",""#
                    ));
                }
                return Ok(p + 1);
            }
            Kind::End => return Err(r#"missing ")""#.into()),
            _ => {
                return Err(if previous.kind == Kind::Ident {
                    format!(
                        r#"expected "," or ")" after "{}", found "{}""#,
                        previous.text, token.text
                    )
                } else {
                    format!(r#"expected a value, found "{}""#, token.text)
                });
            }
        }
        previous = token;
        p += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selector_values() {
        for s in [
            "app=web",
            "app==web",
            "app!=web",
            "app",
            "!app",
            "app in (a, b)",
            "app notin (a)",
            "replicas>3",
            "replicas<07",
            "app=,tier=db",
            "a.b/c=d, e in (f)",
            "app = web",
            "app==",
            "app!=",
            "in=x",
            "notin",
            "app=in",
            "app in (in, notin)",
            "tier notin (x),env",
            "a\tin (c)",
        ] {
            assert_eq!(check_selector(s), Ok(()), "input {s:?}");
        }
    }

    #[test]
    fn selector_errors() {
        let cases = [
            (
                "",
                "an empty selector matches everything; use your API's match-all form instead, e.g. LabelSelector::default()",
            ),
            (" app=web", "remove the spaces around the literal"),
            ("app=web,", r#"remove the trailing ",""#),
            ("app=web,,tier=db", "empty requirement between commas"),
            (",app=web", "empty requirement between commas"),
            (
                "App@=web",
                r#"key "App@": invalid character `@` in the name part; use letters, digits, "-", "_" and ".""#,
            ),
            (
                "app=web app",
                r#"expected "," or the end after "app=web", found "app""#,
            ),
            (
                "app=-web",
                r#"value "-web": label values must start and end with a letter or digit"#,
            ),
            (
                "app in (a,)",
                r#"empty value in "(a,)"; remove the extra ",""#,
            ),
            (
                "app in (,a)",
                r#"empty value in "(,a)"; remove the extra ",""#,
            ),
            (
                "app in (a,,b)",
                r#"empty value in "(a,,b)"; remove the extra ",""#,
            ),
            ("app in ()", r#""in" needs at least one value"#),
            ("app notin ()", r#""notin" needs at least one value"#),
            ("app in (a", r#"missing ")""#),
            ("app in a", r#"expected "(" after "in""#),
            ("app > x", r#""x" after ">" must be a whole number"#),
            ("x<-5", r#""-5" after "<" must be a whole number"#),
            ("x<", r#"expected a whole number after "<""#),
            (
                "!app=web",
                r#""!" checks that a key is absent; write "!app" or "app!=web""#,
            ),
            ("=web", r#"missing key before "=""#),
            (
                "app (a)",
                r#"expected an operator or "," after "app", found "(""#,
            ),
            ("!", r#"expected a key after "!""#),
            ("(app)", r#"expected a key, found "(""#),
            (
                "app in (a b)",
                r#"expected "," or ")" after "a", found "b""#,
            ),
            ("app=(", r#"expected a value after "=", found "(""#),
        ];
        for (input, message) in cases {
            assert_eq!(
                check_selector(input),
                Err(message.to_string()),
                "input {input:?}"
            );
        }
    }

    #[test]
    fn selector_multibyte() {
        assert_eq!(
            check_selector("app=été"),
            Err(
                r#"value "été": invalid character `é`; use letters, digits, "-", "_" and ".""#
                    .to_string()
            )
        );
        for s in [
            "!é",
            "a=(é",
            "a in (é)",
            "a=b,é",
            "é",
            "a!é",
            "a<é",
            "a>=é",
            "a==é",
        ] {
            assert!(check_selector(s).is_err(), "input {s:?}");
        }
    }

    /// Random strings, multi-byte characters included, never panic any k8s parser.
    #[test]
    fn checks_never_panic() {
        let alphabet: Vec<char> = "a9Z-_./=!(),<>@ \tµéñ€😀".chars().collect();
        let mut state = 5u64;
        let mut next = || {
            state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^ (z >> 31)
        };
        for _ in 0..50_000 {
            let len = next() % 12;
            let s: String = (0..len)
                .map(|_| alphabet[(next() % alphabet.len() as u64) as usize])
                .collect();
            let _ = check_selector(&s);
            let _ = crate::k8s_names::check_dns_label(&s);
            let _ = crate::k8s_names::check_dns1035_label(&s);
            let _ = crate::k8s_names::check_dns_subdomain(&s);
            let _ = crate::k8s_names::check_label_key(&s);
            let _ = crate::k8s_names::check_label_value(&s);
            let _ = crate::quantity::parse_quantity(&s);
            let _ = crate::quantity::parse_cpu(&s);
            let _ = crate::quantity::parse_mem(&s);
        }
    }

    #[test]
    fn selector_go_oracle() {
        let fixture = include_str!("testdata/k8s_selector.tsv");
        let (mut compared, mut accepted) = (0, 0);
        for line in fixture.lines().filter(|l| !l.starts_with('#')) {
            let (input, verdict) = line.split_once('\t').unwrap();
            compared += 1;
            match (verdict, check_selector(input)) {
                ("ok", Ok(())) => accepted += 1,
                ("err", Err(_)) => {}
                ("ok", Err(message)) => {
                    // Our strictness, each checked against the input: empty
                    // selectors, empty values in a set and surrounding whitespace,
                    // all of which Kubernetes accepts.
                    let compact: String = input.split_whitespace().collect();
                    let allowed = if message.starts_with("an empty selector") {
                        input.is_empty()
                    } else if message.starts_with("empty value in") {
                        ["(,", ",,", ",)"].iter().any(|p| compact.contains(p))
                    } else if message.ends_with("needs at least one value") {
                        compact.contains("()")
                    } else if message == "remove the spaces around the literal" {
                        input.trim() != input
                    } else {
                        false
                    };
                    assert!(allowed, "Go accepts {input:?}, we say: {message}");
                }
                (_, Ok(())) => panic!("Go rejects {input:?}, we accept it"),
                _ => unreachable!(),
            }
        }
        assert!(accepted >= 500, "only {accepted} accepted fixture lines");
        assert!(compared >= 2000, "only {compared} fixture lines");
    }
}
