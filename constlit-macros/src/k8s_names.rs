//! `k8s::dns_label!`, `dns1035_label!`, `dns_subdomain!`, `label_key!` and
//! `label_value!`: Kubernetes object names and labels.
//!
//! The rules are those of `k8s.io/apimachinery/pkg/util/validation`, which the
//! fixture in `testdata/k8s_names.tsv` pins.

fn around_spaces(s: &str) -> Result<(), String> {
    if s.starts_with(char::is_whitespace) || s.ends_with(char::is_whitespace) {
        return Err("remove the spaces around the literal".into());
    }
    Ok(())
}

fn alnum_ends(s: &str) -> bool {
    let ok = |c: Option<char>| c.is_some_and(|c| c.is_ascii_alphanumeric());
    ok(s.chars().next()) && ok(s.chars().last())
}

/// An RFC 1123 label (`rfc1035: false`) or an RFC 1035 label, which must also
/// start with a letter.
fn dns_label(s: &str, rfc1035: bool) -> Result<(), String> {
    if s.is_empty() {
        return Err(r#"empty name; write e.g. "my-app""#.into());
    }
    around_spaces(s)?;
    // Upper case is reported last, once every other rule holds, so the
    // suggested lower-case name is valid.
    if let Some(c) = s
        .chars()
        .find(|&c| !(c.is_ascii_alphanumeric() || c == '-'))
    {
        let hint = if c == '.' {
            " (dns_subdomain! allows dots)"
        } else {
            ""
        };
        return Err(format!(
            r#"invalid character `{c}`; use lower-case letters, digits and "-"{hint}"#
        ));
    }
    if s.len() > 63 {
        return Err(format!(
            "names are at most 63 characters; this one has {}",
            s.len()
        ));
    }
    if rfc1035 && !s.starts_with(|c: char| c.is_ascii_alphabetic()) {
        return Err("this name must start with a letter (RFC 1035)".into());
    }
    if !alnum_ends(s) {
        return Err("names must start and end with a letter or digit".into());
    }
    lower_case(s)
}

/// The last check of a DNS name: every other rule holds, so only case is left.
fn lower_case(s: &str) -> Result<(), String> {
    let lower = s.to_ascii_lowercase();
    if lower != s {
        return Err(format!(r#"names must be lower case; write "{lower}""#));
    }
    Ok(())
}

/// `k8s::dns_label!`: an RFC 1123 label, like namespace and most object names.
pub fn check_dns_label(s: &str) -> Result<(), String> {
    dns_label(s, false)
}

/// `k8s::dns1035_label!`: an RFC 1035 label, like Service names.
pub fn check_dns1035_label(s: &str) -> Result<(), String> {
    dns_label(s, true)
}

/// `k8s::dns_subdomain!`: dot-separated RFC 1123 labels, at most 253 characters.
pub fn check_dns_subdomain(s: &str) -> Result<(), String> {
    if s.is_empty() {
        return Err(r#"empty name; write e.g. "my-app""#.into());
    }
    around_spaces(s)?;
    if let Some(c) = s
        .chars()
        .find(|&c| !(c.is_ascii_alphanumeric() || c == '-' || c == '.'))
    {
        return Err(format!(
            r#"invalid character `{c}`; use lower-case letters, digits, "-" and ".""#
        ));
    }
    if s.len() > 253 {
        return Err(format!(
            "names are at most 253 characters; this one has {}",
            s.len()
        ));
    }
    let parts: Vec<&str> = s.split('.').collect();
    if parts.iter().any(|p| p.is_empty()) {
        return Err("empty part between dots".into());
    }
    if let Some(part) = parts.iter().find(|p| !alnum_ends(p)) {
        return Err(if parts.len() == 1 {
            "names must start and end with a letter or digit".into()
        } else {
            format!(r#"part "{part}" must start and end with a letter or digit"#)
        });
    }
    lower_case(s)
}

/// The name part of a label key, or a label value: at most 63 characters of
/// letters, digits, `-`, `_` and `.`, starting and ending with a letter or digit.
fn qualified(s: &str, subject: (&str, &str), of: &str) -> Result<(), String> {
    let (is, must) = subject;
    if let Some(c) = s
        .chars()
        .find(|&c| !(c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')))
    {
        return Err(format!(
            r#"invalid character `{c}`{of}; use letters, digits, "-", "_" and ".""#
        ));
    }
    if s.len() > 63 {
        return Err(format!(
            "{is} at most 63 characters; this one has {}",
            s.len()
        ));
    }
    if !alnum_ends(s) {
        return Err(format!("{must} start and end with a letter or digit"));
    }
    Ok(())
}

/// `k8s::label_key!`: an optional DNS-subdomain prefix and `/`, then a name.
pub fn check_label_key(s: &str) -> Result<(), String> {
    if s.is_empty() {
        return Err(r#"empty label key; write e.g. "app.kubernetes.io/name""#.into());
    }
    around_spaces(s)?;
    if s.matches('/').count() > 1 {
        return Err(r#"a label key has at most one "/""#.into());
    }
    let name = match s.split_once('/') {
        Some((prefix, name)) => {
            if prefix.is_empty() {
                return Err(r#"empty prefix before "/""#.into());
            }
            if name.is_empty() {
                return Err(r#"missing name after "/""#.into());
            }
            check_dns_subdomain(prefix).map_err(|e| format!(r#"prefix "{prefix}": {e}"#))?;
            name
        }
        None => s,
    };
    qualified(
        name,
        ("the name part is", "the name part must"),
        " in the name part",
    )
}

/// `k8s::label_value!`: empty, or a name as in a label key.
pub fn check_label_value(s: &str) -> Result<(), String> {
    if s.is_empty() {
        return Ok(());
    }
    around_spaces(s)?;
    qualified(s, ("label values are", "label values must"), "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_values() {
        let ok = |result: Result<(), String>, input: &str| {
            assert_eq!(result, Ok(()), "input {input:?}");
        };
        for s in ["my-app", "a", "0", &"a".repeat(63)] {
            ok(check_dns_label(s), s);
        }
        ok(check_dns1035_label("web-1"), "web-1");
        let long = format!("{}.{}", "a".repeat(200), "b".repeat(52));
        for s in ["app.config.v1", &long] {
            ok(check_dns_subdomain(s), s);
        }
        for s in [
            "app.kubernetes.io/name",
            "App.Kubernetes",
            "example.com/Web_App-1.x",
        ] {
            ok(check_label_key(s), s);
        }
        for s in ["", "Web", "v1.2_3"] {
            ok(check_label_value(s), s);
        }
    }

    #[test]
    fn name_errors() {
        type Check = fn(&str) -> Result<(), String>;
        let label: Check = check_dns_label;
        let rfc1035: Check = check_dns1035_label;
        let subdomain: Check = check_dns_subdomain;
        let key: Check = check_label_key;
        let value: Check = check_label_value;
        let dns_chars = r#"use lower-case letters, digits and "-""#;
        let a64 = "a".repeat(64);
        let a254 = "a".repeat(254);
        let key64 = "k".repeat(64);
        let cases: Vec<(Check, &str, String)> =
            vec![
            (label, "", r#"empty name; write e.g. "my-app""#.into()),
            (rfc1035, "", r#"empty name; write e.g. "my-app""#.into()),
            (subdomain, "", r#"empty name; write e.g. "my-app""#.into()),
            (label, "My-App", r#"names must be lower case; write "my-app""#.into()),
            (rfc1035, "My-App", r#"names must be lower case; write "my-app""#.into()),
            (subdomain, "My.App", r#"names must be lower case; write "my.app""#.into()),
            (label, "my_app", format!("invalid character `_`; {dns_chars}")),
            (rfc1035, "my_app", format!("invalid character `_`; {dns_chars}")),
            (
                label,
                "my.app",
                format!("invalid character `.`; {dns_chars} (dns_subdomain! allows dots)"),
            ),
            (label, "-app", "names must start and end with a letter or digit".into()),
            (label, "app-", "names must start and end with a letter or digit".into()),
            (subdomain, "-app", "names must start and end with a letter or digit".into()),
            (rfc1035, "1web", "this name must start with a letter (RFC 1035)".into()),
            (rfc1035, "web-", "names must start and end with a letter or digit".into()),
            (label, &a64, "names are at most 63 characters; this one has 64".into()),
            (rfc1035, &a64, "names are at most 63 characters; this one has 64".into()),
            (subdomain, &a254, "names are at most 253 characters; this one has 254".into()),
            (subdomain, "app..config", "empty part between dots".into()),
            (subdomain, ".app", "empty part between dots".into()),
            (subdomain, "app.", "empty part between dots".into()),
            (
                subdomain,
                "app.-x",
                r#"part "-x" must start and end with a letter or digit"#.into(),
            ),
            (
                subdomain,
                "app.my_x",
                r#"invalid character `_`; use lower-case letters, digits, "-" and ".""#.into(),
            ),
            (key, "", r#"empty label key; write e.g. "app.kubernetes.io/name""#.into()),
            (key, "a/b/c", r#"a label key has at most one "/""#.into()),
            (key, "/name", r#"empty prefix before "/""#.into()),
            (key, "example.com/", r#"missing name after "/""#.into()),
            (
                key,
                "Example.com/name",
                r#"prefix "Example.com": names must be lower case; write "example.com""#.into(),
            ),
            (
                key,
                &key64,
                "the name part is at most 63 characters; this one has 64".into(),
            ),
            (
                key,
                "_app",
                "the name part must start and end with a letter or digit".into(),
            ),
            (
                key,
                "app.",
                "the name part must start and end with a letter or digit".into(),
            ),
            (
                key,
                "app@1",
                r#"invalid character `@` in the name part; use letters, digits, "-", "_" and ".""#
                    .into(),
            ),
            (
                value,
                &a64,
                "label values are at most 63 characters; this one has 64".into(),
            ),
            (
                value,
                "-web",
                "label values must start and end with a letter or digit".into(),
            ),
            (
                value,
                "web_",
                "label values must start and end with a letter or digit".into(),
            ),
            (
                value,
                "web app",
                r#"invalid character ` `; use letters, digits, "-", "_" and ".""#.into(),
            ),
            (label, " app", "remove the spaces around the literal".into()),
            (value, "web ", "remove the spaces around the literal".into()),
            (label, "My_App", format!("invalid character `_`; {dns_chars}")),
            (label, "-App", "names must start and end with a letter or digit".into()),
            (subdomain, "My_App.x", r#"invalid character `_`; use lower-case letters, digits, "-" and ".""#.into()),
            (
                key,
                "Ex_ample.com/name",
                r#"prefix "Ex_ample.com": invalid character `_`; use lower-case letters, digits, "-" and ".""#.into(),
            ),
        ];
        for (check, input, message) in cases {
            assert_eq!(check(input), Err(message), "input {input:?}");
        }
    }

    #[test]
    fn names_go_oracle() {
        let fixture = include_str!("testdata/k8s_names.tsv");
        let mut compared = 0;
        let mut accepted = std::collections::HashMap::new();
        for line in fixture.lines().filter(|l| !l.starts_with('#')) {
            let fields: Vec<&str> = line.split('\t').collect();
            let (kind, input, verdict) = (fields[0], fields[1], fields[2]);
            if verdict == "ok" {
                *accepted.entry(kind).or_insert(0) += 1;
            }
            let ours = match kind {
                "dns_label" => check_dns_label(input),
                "dns1035_label" => check_dns1035_label(input),
                "dns_subdomain" => check_dns_subdomain(input),
                "label_key" => check_label_key(input),
                "label_value" => check_label_value(input),
                _ => panic!("unknown kind {kind}"),
            };
            compared += 1;
            match (verdict, ours) {
                ("ok", Ok(())) | ("err", Err(_)) => {}
                ("ok", Err(message)) => assert_eq!(
                    message, "remove the spaces around the literal",
                    "Go accepts {kind} {input:?}"
                ),
                (_, Ok(())) => panic!("Go rejects {kind} {input:?}, we accept it"),
                _ => unreachable!(),
            }
        }
        assert!(compared >= 2000, "only {compared} fixture lines");
        // Both directions need real coverage: Go-accepted inputs too.
        for kind in [
            "dns_label",
            "dns1035_label",
            "dns_subdomain",
            "label_key",
            "label_value",
        ] {
            let n = accepted.get(kind).copied().unwrap_or(0);
            assert!(n >= 150, "only {n} accepted {kind} lines");
        }
    }
}
