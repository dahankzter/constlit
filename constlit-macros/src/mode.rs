//! `unix::mode!`: file permission bits as octal, `ls` or absolute symbolic modes.

const SETUID: u32 = 0o4000;
const SETGID: u32 = 0o2000;
const STICKY: u32 = 0o1000;

/// Parses a permission mode into its bits (`<= 0o7777`).
pub fn parse_mode(s: &str) -> Result<u32, String> {
    if s.is_empty() {
        return Err(r#"empty mode; write octal like "0755", or "rwxr-xr-x""#.into());
    }
    if s.starts_with(char::is_whitespace) || s.ends_with(char::is_whitespace) {
        return Err("remove the spaces around the literal".into());
    }
    if s.contains(char::is_whitespace) {
        let joined: String = s.split_whitespace().collect();
        // Offer the joined mode only when it is valid; otherwise report its problem.
        parse_mode(&joined)?;
        return Err(format!(r#"remove the spaces: "{joined}""#));
    }
    if let Some(rest) = s.strip_prefix("0o") {
        let bits = parse_octal(rest)?;
        return Err(format!(r#"write the mode without "0o": "{bits:04o}""#));
    }
    if s.bytes().all(|b| b.is_ascii_digit()) {
        return parse_octal(s);
    }
    // A line of `ls -l` output: a file type before the permissions, and maybe an
    // ACL (`+`), SELinux (`.`) or extended-attribute (`@`) marker after them.
    let (permissions, marker) = match s.char_indices().nth(9) {
        Some((i, c)) if s.len() == i + 1 && matches!(c, '+' | '.' | '@') => (&s[..i], Some(c)),
        _ => (s, None),
    };
    if let Some(kind) = permissions
        .chars()
        .next()
        .filter(|c| "-dlcbpsD".contains(*c))
    {
        let tail = &permissions[1..];
        let tail_marker = match s[1..].char_indices().nth(9) {
            Some((i, c)) if s.len() == i + 2 && matches!(c, '+' | '.' | '@') => {
                Some((&s[1..=i], c))
            }
            _ => None,
        };
        if marker.is_none() && tail.chars().count() == 9 && parse_ls(tail).is_ok() {
            return Err(format!(r#"remove the file type "{kind}": "{tail}""#));
        }
        if let Some((tail, marker)) = tail_marker.filter(|(tail, _)| parse_ls(tail).is_ok()) {
            return Err(format!(
                r#"remove the file type "{kind}" and the trailing "{marker}": "{tail}""#
            ));
        }
    }
    if let Some(marker) = marker.filter(|_| parse_ls(permissions).is_ok()) {
        return Err(format!(
            r#"remove the trailing "{marker}": "{permissions}""#
        ));
    }
    // A valid ls form like "-wx------" also reads as the relative clause "-wx...".
    if let Ok(bits) = parse_ls(s) {
        return Ok(bits);
    }
    let symbolic = s.contains(['=', '+']) || s.split(',').any(is_relative);
    if symbolic {
        parse_symbolic(s)
    } else {
        parse_ls(s)
    }
}

/// A relative clause like `"u+x"`, `"go-w"` or `"+x"` (classes, `+` or `-`, permissions).
fn is_relative(clause: &str) -> bool {
    let rest = clause.trim_start_matches(['u', 'g', 'o', 'a']);
    rest.starts_with(['+', '-'])
        && rest[1..]
            .chars()
            .all(|c| matches!(c, 'r' | 'w' | 'x' | 's' | 't'))
}

/// `"755"`, `"0755"`, `"4755"`: 3 or 4 octal digits.
fn parse_octal(s: &str) -> Result<u32, String> {
    if let Some(c) = s.chars().find(|c| !matches!(c, '0'..='7')) {
        return Err(format!("digit `{c}` is not octal (0-7)"));
    }
    if s.len() < 3 {
        return Err(r#"write 3 or 4 octal digits, like "0755""#.into());
    }
    let significant = s.trim_start_matches('0');
    if significant.len() > 4 {
        return Err(format!("mode {s} is out of range (the highest is 7777)"));
    }
    let bits = u32::from_str_radix(
        if significant.is_empty() {
            "0"
        } else {
            significant
        },
        8,
    )
    .expect("checked octal");
    if s.len() > 4 {
        return Err(format!(r#"write at most 4 octal digits: "{bits:04o}""#));
    }
    Ok(bits)
}

/// `"rwxr-xr-x"`, with `s`/`S` for setuid and setgid and `t`/`T` for sticky.
fn parse_ls(s: &str) -> Result<u32, String> {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() != 9 {
        return Err(r#"write all 9 permission characters, like "rwxr-xr-x""#.into());
    }
    let mut bits = 0;
    for (i, &c) in chars.iter().enumerate() {
        let shift = 6 - 3 * (i / 3) as u32;
        let special = [SETUID, SETGID, STICKY][i / 3];
        let (set, unset) = if i / 3 == 2 { ('t', 'T') } else { ('s', 'S') };
        let (bit, expected) = match i % 3 {
            0 => (4, "`r` or `-`".to_string()),
            1 => (2, "`w` or `-`".to_string()),
            _ => (1, format!("`x`, `{set}`, `{unset}` or `-`")),
        };
        bits |= match c {
            '-' => 0,
            'r' if bit == 4 => 4 << shift,
            'w' if bit == 2 => 2 << shift,
            'x' if bit == 1 => 1 << shift,
            _ if bit == 1 && c == set => (1 << shift) | special,
            _ if bit == 1 && c == unset => special,
            _ => {
                return Err(format!(
                    "unexpected `{c}` at position {}; expected {expected}",
                    i + 1
                ));
            }
        };
    }
    Ok(bits)
}

/// `"u=rwx,go=rx"`: absolute clauses only. Classes not named get no bits.
fn parse_symbolic(s: &str) -> Result<u32, String> {
    let mut bits = 0;
    let mut seen = [false; 3];
    for clause in s.split(',') {
        if clause.is_empty() {
            return Err(r#"empty clause; remove the extra ",""#.into());
        }
        let Some(op) = clause.find(['=', '+', '-']) else {
            return Err(format!(
                r#"missing "=" in "{clause}"; write e.g. "u=rwx,go=rx""#
            ));
        };
        let (who, perms) = (&clause[..op], &clause[op + 1..]);
        if let Some(c) = who.chars().find(|c| !matches!(c, 'u' | 'g' | 'o' | 'a')) {
            return Err(format!("unknown class `{c}`; use u, g, o or a"));
        }
        if !clause[op..].starts_with('=') {
            return Err(format!(
                r#""{clause}" changes a mode; write an absolute mode like "u=rwx,go=rx" or "0755""#
            ));
        }
        if who.is_empty() {
            return Err(r#"missing class before "="; write e.g. "a=r""#.into());
        }
        if let Some(c) = perms
            .chars()
            .find(|c| !matches!(c, 'r' | 'w' | 'x' | 's' | 't'))
        {
            return Err(format!("unknown permission `{c}`; use r, w, x, s or t"));
        }
        let classes: Vec<usize> = who
            .chars()
            .flat_map(|c| match c {
                'u' => &[0][..],
                'g' => &[1][..],
                'o' => &[2][..],
                _ => &[0, 1, 2][..],
            })
            .copied()
            .collect();
        if perms.contains('s') && !classes.iter().any(|&c| c < 2) {
            return Err(r#""s" (setuid and setgid) applies only to u, g or a"#.into());
        }
        if perms.contains('t') && !classes.contains(&2) {
            return Err(r#""t" (sticky) applies only to o or a"#.into());
        }
        for class in classes {
            if seen[class] {
                let name = ["u", "g", "o"][class];
                return Err(format!(
                    r#""{name}" is set twice; set each of u, g and o once"#
                ));
            }
            seen[class] = true;
            let shift = 6 - 3 * class as u32;
            for c in perms.chars() {
                bits |= match (c, class) {
                    ('r', _) => 4 << shift,
                    ('w', _) => 2 << shift,
                    ('x', _) => 1 << shift,
                    ('s', 0) => SETUID,
                    ('s', 1) => SETGID,
                    ('t', 2) => STICKY,
                    _ => 0,
                };
            }
        }
    }
    Ok(bits)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn err(s: &str) -> String {
        parse_mode(s).expect_err(s)
    }

    #[test]
    fn mode_values() {
        let cases = [
            ("755", 0o755),
            ("0755", 0o755),
            ("4755", 0o4755),
            ("1777", 0o1777),
            ("000", 0),
            ("rwxr-xr-x", 0o755),
            ("rwsr-xr-x", 0o4755),
            ("rwSr--r--", 0o4644),
            ("rwxr-sr-x", 0o2755),
            ("rwxrwxrwt", 0o1777),
            ("rw-r--r-T", 0o1644),
            ("---------", 0),
            ("u=rwx,g=rx,o=rx", 0o755),
            ("u=rwx,go=rx", 0o755),
            ("u=rwx", 0o700),
            ("a=", 0),
            ("a=rwxst", 0o7777),
            ("u=rwxs,g=x", 0o4710),
            ("o=t", 0o1000),
            ("ug=rw,o=r", 0o664),
        ];
        for (input, bits) in cases {
            assert_eq!(parse_mode(input), Ok(bits), "input {input:?}");
        }
    }

    #[test]
    fn mode_errors() {
        let cases = [
            ("", r#"empty mode; write octal like "0755", or "rwxr-xr-x""#),
            ("0o755", r#"write the mode without "0o": "0755""#),
            ("75", r#"write 3 or 4 octal digits, like "0755""#),
            ("7", r#"write 3 or 4 octal digits, like "0755""#),
            ("00755", r#"write at most 4 octal digits: "0755""#),
            ("17777", "mode 17777 is out of range (the highest is 7777)"),
            ("0758", "digit `8` is not octal (0-7)"),
            ("-rwxr-xr-x", r#"remove the file type "-": "rwxr-xr-x""#),
            ("drwxr-xr-x", r#"remove the file type "d": "rwxr-xr-x""#),
            (
                "rwxr-xr",
                r#"write all 9 permission characters, like "rwxr-xr-x""#,
            ),
            (
                "rwxr-xr-q",
                "unexpected `q` at position 9; expected `x`, `t`, `T` or `-`",
            ),
            (
                "wwxr-xr-x",
                "unexpected `w` at position 1; expected `r` or `-`",
            ),
            (
                "rxxr-xr-x",
                "unexpected `x` at position 2; expected `w` or `-`",
            ),
            (
                "rwxr-tr-x",
                "unexpected `t` at position 6; expected `x`, `s`, `S` or `-`",
            ),
            (
                "u+x",
                r#""u+x" changes a mode; write an absolute mode like "u=rwx,go=rx" or "0755""#,
            ),
            (
                "go-w",
                r#""go-w" changes a mode; write an absolute mode like "u=rwx,go=rx" or "0755""#,
            ),
            (
                "u=rwx,u=r",
                r#""u" is set twice; set each of u, g and o once"#,
            ),
            (
                "a=r,u=w",
                r#""u" is set twice; set each of u, g and o once"#,
            ),
            (
                "o=s",
                r#""s" (setuid and setgid) applies only to u, g or a"#,
            ),
            ("u=t", r#""t" (sticky) applies only to o or a"#),
            ("x=r", "unknown class `x`; use u, g, o or a"),
            ("u=rq", "unknown permission `q`; use r, w, x, s or t"),
            ("=r", r#"missing class before "="; write e.g. "a=r""#),
            ("u=rwx,", r#"empty clause; remove the extra ",""#),
            ("u=rwx,,g=r", r#"empty clause; remove the extra ",""#),
            ("u=rwx, g=rx", r#"remove the spaces: "u=rwx,g=rx""#),
            (" 0755", "remove the spaces around the literal"),
            (
                "+x",
                r#""+x" changes a mode; write an absolute mode like "u=rwx,go=rx" or "0755""#,
            ),
            (
                "-x",
                r#""-x" changes a mode; write an absolute mode like "u=rwx,go=rx" or "0755""#,
            ),
            (
                "u=rwx,+x",
                r#""+x" changes a mode; write an absolute mode like "u=rwx,go=rx" or "0755""#,
            ),
            (
                "-rwxr-xr-x+",
                r#"remove the file type "-" and the trailing "+": "rwxr-xr-x""#,
            ),
            (
                "drwxr-xr-x.",
                r#"remove the file type "d" and the trailing ".": "rwxr-xr-x""#,
            ),
            (
                "-rwxr-xr-x@",
                r#"remove the file type "-" and the trailing "@": "rwxr-xr-x""#,
            ),
            ("rwxr-xr-x+", r#"remove the trailing "+": "rwxr-xr-x""#),
            (
                "=rwxr-xr-x",
                r#"missing class before "="; write e.g. "a=r""#,
            ),
            (
                "u+x, g=r",
                r#""u+x" changes a mode; write an absolute mode like "u=rwx,go=rx" or "0755""#,
            ),
        ];
        for (input, message) in cases {
            assert_eq!(err(input), message, "input {input:?}");
        }
    }

    /// The `ls` spelling of `bits`, built independently of the parser.
    fn ls(bits: u32) -> String {
        let mut s = String::new();
        for (class, special, special_set, special_unset) in [
            (6, 0o4000, 's', 'S'),
            (3, 0o2000, 's', 'S'),
            (0, 0o1000, 't', 'T'),
        ] {
            let perm = (bits >> class) & 7;
            s.push(if perm & 4 != 0 { 'r' } else { '-' });
            s.push(if perm & 2 != 0 { 'w' } else { '-' });
            s.push(match (perm & 1 != 0, bits & special != 0) {
                (true, true) => special_set,
                (false, true) => special_unset,
                (true, false) => 'x',
                (false, false) => '-',
            });
        }
        s
    }

    /// A symbolic spelling of `bits`: `u=…,g=…,o=…`.
    fn symbolic(bits: u32) -> String {
        let mut clauses = Vec::new();
        for (who, class, special, letter) in [
            ('u', 6, 0o4000, 's'),
            ('g', 3, 0o2000, 's'),
            ('o', 0, 0o1000, 't'),
        ] {
            let perm = (bits >> class) & 7;
            let mut clause = format!("{who}=");
            for (bit, c) in [(4, 'r'), (2, 'w'), (1, 'x')] {
                if perm & bit != 0 {
                    clause.push(c);
                }
            }
            if bits & special != 0 {
                clause.push(letter);
            }
            clauses.push(clause);
        }
        clauses.join(",")
    }

    #[test]
    fn mode_forms_agree() {
        for bits in 0..=0o7777 {
            for text in [format!("{bits:04o}"), ls(bits), symbolic(bits)] {
                assert_eq!(parse_mode(&text), Ok(bits), "input {text:?}");
            }
        }
    }
}
