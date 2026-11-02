//! `systemd::unit!`: systemd unit names, by systemd's `unit_name_is_valid`.

use crate::suggest;

const TYPES: [&str; 11] = [
    "service",
    "socket",
    "device",
    "mount",
    "automount",
    "swap",
    "target",
    "path",
    "timer",
    "slice",
    "scope",
];

/// systemd's `VALID_CHARS`: letters, digits and `:-_.\`.
fn valid_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, ':' | '-' | '_' | '.' | '\\')
}

/// Checks `prefix[@instance].type`; a template has an empty instance.
pub fn check_unit(s: &str) -> Result<(), String> {
    if s.is_empty() {
        return Err(r#"empty unit name; write e.g. "nginx.service""#.into());
    }
    if s.starts_with(char::is_whitespace) || s.ends_with(char::is_whitespace) {
        return Err("remove the spaces around the literal".into());
    }
    let Some((name, kind)) = s.rsplit_once('.') else {
        return Err(format!(r#"missing unit type; write e.g. "{s}.service""#));
    };
    if kind.is_empty() {
        return Err(r#"missing unit type after ".""#.into());
    }
    if !TYPES.contains(&kind) {
        return Err(match suggest::closest(kind, TYPES) {
            Some(near) => format!(r#"unknown unit type ".{kind}"; did you mean ".{near}"?"#),
            None => format!(
                r#"unknown unit type ".{kind}"; use service, socket, device, mount, automount, swap, target, path, timer, slice or scope"#
            ),
        });
    }
    let (prefix, instance) = match name.split_once('@') {
        Some((prefix, instance)) => (prefix, Some(instance)),
        None => (name, None),
    };
    if prefix.is_empty() {
        let at = if instance.is_some() { "@" } else { "." };
        return Err(format!(r#"missing unit name before "{at}""#));
    }
    // The instance may contain "@" (systemd's VALID_CHARS_WITH_AT).
    let bad = prefix
        .chars()
        .find(|&c| !valid_char(c))
        .or_else(|| instance.and_then(|i| i.chars().find(|&c| !valid_char(c) && c != '@')));
    if let Some(c) = bad {
        return Err(format!("invalid character `{c}` in a unit name"));
    }
    if s.len() > 255 {
        return Err(format!(
            "unit names are at most 255 characters; this one has {}",
            s.len()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_values() {
        for name in [
            "foo.service",
            "foo@.service",
            "foo@bar.service",
            "foo@bar@baz.service",
            r"a.b-c_d:e\x2d.mount",
            "-.slice",
            "getty@tty1.service",
            "x.socket",
            "x.device",
            "x.mount",
            "x.automount",
            "x.swap",
            "x.target",
            "x.path",
            "x.timer",
            "x.slice",
            "x.scope",
        ] {
            assert_eq!(check_unit(name), Ok(()), "{name}");
        }
    }

    #[test]
    fn unit_errors() {
        let long = format!("{}.service", "a".repeat(248));
        let cases = [
            ("", r#"empty unit name; write e.g. "nginx.service""#.to_string()),
            (" foo.service", "remove the spaces around the literal".into()),
            ("nginx", r#"missing unit type; write e.g. "nginx.service""#.into()),
            ("nginx.servcie", r#"unknown unit type ".servcie"; did you mean ".service"?"#.into()),
            (
                "nginx.daemon",
                r#"unknown unit type ".daemon"; use service, socket, device, mount, automount, swap, target, path, timer, slice or scope"#.into(),
            ),
            (".service", r#"missing unit name before ".""#.into()),
            ("@.service", r#"missing unit name before "@""#.into()),
            ("@x.service", r#"missing unit name before "@""#.into()),
            ("ng/inx.service", "invalid character `/` in a unit name".into()),
            ("foo bar.service", "invalid character ` ` in a unit name".into()),
            ("foo@b/r.service", "invalid character `/` in a unit name".into()),
            ("foo@bar.", r#"missing unit type after ".""#.into()),
            ("foo.service.", r#"missing unit type after ".""#.into()),
            (long.as_str(), "unit names are at most 255 characters; this one has 256".into()),
        ];
        for (input, message) in cases {
            assert_eq!(check_unit(input), Err(message), "input {input:?}");
        }
    }
}
