//! `linux::ifname!` and `linux::dev!`: network interface names and device numbers.

/// The kernel's `dev_valid_name`, narrowed to printable ASCII and without `%`
/// (which makes a name template, like `eth%d`).
pub fn check_ifname(s: &str) -> Result<(), String> {
    if s.is_empty() {
        return Err(r#"empty interface name; write e.g. "eth0""#.into());
    }
    if s.starts_with(char::is_whitespace) || s.ends_with(char::is_whitespace) {
        return Err("remove the spaces around the literal".into());
    }
    if s == "." || s == ".." {
        return Err(format!(r#""{s}" can't be an interface name"#));
    }
    if let Some(c) = s
        .chars()
        .find(|&c| matches!(c, '/' | ':') || c.is_whitespace())
    {
        return Err(format!("invalid character `{c}` in an interface name"));
    }
    if s.contains('%') {
        return Err(r#""%" makes a name template; write a concrete name like "eth0""#.into());
    }
    if let Some(c) = s.chars().find(|c| !c.is_ascii_graphic()) {
        return Err(format!(
            "invalid character `{c}`; interface names here are printable ASCII"
        ));
    }
    if s.len() > 15 {
        return Err(format!(
            "interface names are at most 15 bytes; this one has {}",
            s.len()
        ));
    }
    Ok(())
}

/// The kernel's limits on device numbers (`MKDEV`): 12-bit majors, 20-bit minors.
const MAJOR_MAX: u32 = 4095;
const MINOR_MAX: u32 = 1_048_575;

/// Parses `"major:minor"` in decimal.
pub fn parse_dev(s: &str) -> Result<(u32, u32), String> {
    if s.is_empty() {
        return Err(r#"empty device number; write "major:minor", like "8:0""#.into());
    }
    if s.starts_with(char::is_whitespace) || s.ends_with(char::is_whitespace) {
        return Err("remove the spaces around the literal".into());
    }
    let Some((major, minor)) = s.split_once(':').filter(|(_, minor)| !minor.contains(':')) else {
        return Err(r#"write "major:minor", like "8:0""#.into());
    };
    let number = |text: &str, what: &str, max: u32| -> Result<u32, String> {
        if text.is_empty() {
            return Err(format!(
                r#"missing {what} number; write "major:minor", like "8:0""#
            ));
        }
        if !text.bytes().all(|b| b.is_ascii_digit()) {
            return Err(format!("{what} number `{text}` is not a number"));
        }
        match text.parse::<u32>() {
            Ok(n) if n <= max => Ok(n),
            _ => Err(format!("{what} number {text} is out of range (0-{max})")),
        }
    };
    let (major_n, minor_n) = (
        number(major, "major", MAJOR_MAX)?,
        number(minor, "minor", MINOR_MAX)?,
    );
    let zeros = |t: &str| t.len() > 1 && t.starts_with('0');
    if zeros(major) || zeros(minor) {
        return Err(format!(
            r#"write the numbers without leading zeros: "{major_n}:{minor_n}""#
        ));
    }
    Ok((major_n, minor_n))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ifname_values() {
        let fifteen = "x".repeat(15);
        for s in [
            "eth0", "enp3s0", "wlan0", "br-1a2b", "veth.100", "lo", "a", &fifteen,
        ] {
            assert_eq!(check_ifname(s), Ok(()), "input {s:?}");
        }
    }

    #[test]
    fn ifname_errors() {
        let sixteen = "x".repeat(16);
        let cases = [
            ("", r#"empty interface name; write e.g. "eth0""#),
            (" eth0", "remove the spaces around the literal"),
            (
                &sixteen,
                "interface names are at most 15 bytes; this one has 16",
            ),
            (".", r#""." can't be an interface name"#),
            ("..", r#"".." can't be an interface name"#),
            ("eth/0", "invalid character `/` in an interface name"),
            ("eth:0", "invalid character `:` in an interface name"),
            ("et h0", "invalid character ` ` in an interface name"),
            (
                "eth%d",
                r#""%" makes a name template; write a concrete name like "eth0""#,
            ),
            (
                "ethé",
                "invalid character `é`; interface names here are printable ASCII",
            ),
        ];
        for (input, message) in cases {
            assert_eq!(
                check_ifname(input),
                Err(message.to_string()),
                "input {input:?}"
            );
        }
    }

    #[test]
    fn dev_values() {
        assert_eq!(parse_dev("8:0"), Ok((8, 0)));
        assert_eq!(parse_dev("0:0"), Ok((0, 0)));
        assert_eq!(parse_dev("4095:1048575"), Ok((4095, 1_048_575)));
    }

    #[test]
    fn dev_errors() {
        let cases = [
            (
                "",
                r#"empty device number; write "major:minor", like "8:0""#,
            ),
            (" 8:0", "remove the spaces around the literal"),
            ("8", r#"write "major:minor", like "8:0""#),
            ("8:x", "minor number `x` is not a number"),
            ("x:0", "major number `x` is not a number"),
            ("4096:0", "major number 4096 is out of range (0-4095)"),
            (
                "0:1048576",
                "minor number 1048576 is out of range (0-1048575)",
            ),
            ("08:0", r#"write the numbers without leading zeros: "8:0""#),
            ("8:00", r#"write the numbers without leading zeros: "8:0""#),
            ("8:0:1", r#"write "major:minor", like "8:0""#),
            (
                ":0",
                r#"missing major number; write "major:minor", like "8:0""#,
            ),
            (
                "8:",
                r#"missing minor number; write "major:minor", like "8:0""#,
            ),
        ];
        for (input, message) in cases {
            assert_eq!(
                parse_dev(input),
                Err(message.to_string()),
                "input {input:?}"
            );
        }
    }
}
