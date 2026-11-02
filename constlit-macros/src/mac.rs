//! `mac!`: MAC addresses (EUI-48), six hex bytes separated by `:` or `-`.

const EXAMPLE: &str = "00:1a:2b:3c:4d:5e";

/// Parses a MAC address written as six two-digit hex bytes separated by `:` or `-`.
pub fn parse_mac(s: &str) -> Result<[u8; 6], String> {
    if s.is_empty() {
        return Err(format!(
            r#"empty MAC address; write six hex bytes, like "{EXAMPLE}""#
        ));
    }
    if s.starts_with(char::is_whitespace) || s.ends_with(char::is_whitespace) {
        return Err("remove the spaces around the literal".into());
    }
    if s.contains('.') {
        // Cisco's "001a.2b3c.4d5e": three groups of four hex digits.
        let groups: Vec<&str> = s.split('.').collect();
        if groups.len() == 3 && groups.iter().all(|g| g.len() == 4 && is_hex(g)) {
            let digits = groups.concat();
            return Err(format!(
                r#"write the address as six bytes: "{}""#,
                join_pairs(&digits.to_ascii_lowercase(), ':')
            ));
        }
        return Err(format!(
            r#"write the address as six bytes, like "{EXAMPLE}""#
        ));
    }
    // "001a-2b3c-4d5e": three groups of four with a separator.
    let groups: Vec<&str> = s.split([':', '-']).collect();
    if groups.len() == 3 && groups.iter().all(|g| g.len() == 4 && is_hex(g)) {
        let separator = if s.contains('-') { '-' } else { ':' };
        return Err(format!(
            r#"write the address as six bytes: "{}""#,
            join_pairs(&groups.concat().to_ascii_lowercase(), separator)
        ));
    }
    let separator = s.chars().find(|&c| c == ':' || c == '-');
    let Some(separator) = separator else {
        if s.len() == 12 && is_hex(s) {
            return Err(format!(
                r#"add separators: "{}""#,
                join_pairs(&s.to_ascii_lowercase(), ':')
            ));
        }
        return Err(format!(
            r#"a MAC address has 6 bytes separated by ":" or "-", like "{EXAMPLE}""#
        ));
    };
    let parts: Vec<&str> = s.split([':', '-']).collect();
    let mixed = s.contains(':') && s.contains('-');
    if parts.len() != 6 {
        return Err(format!("a MAC address has 6 bytes, found {}", parts.len()));
    }
    if parts.iter().any(|p| p.is_empty()) {
        return Err(format!(
            r#"empty byte between separators; write six bytes, like "{EXAMPLE}""#
        ));
    }
    if let Some(part) = parts.iter().find(|p| !is_hex(p)) {
        return Err(format!("byte `{part}` is not hex"));
    }
    if let Some(part) = parts.iter().find(|p| p.len() > 2) {
        return Err(format!("byte `{part}` has more than two hex digits"));
    }
    let padded = || {
        parts
            .iter()
            .map(|p| format!("{:0>2}", p.to_ascii_lowercase()))
            .collect::<Vec<_>>()
            .join(&separator.to_string())
    };
    if mixed {
        return Err(format!(r#"use one separator throughout: "{}""#, padded()));
    }
    if parts.iter().any(|p| p.len() == 1) {
        return Err(format!(
            r#"write each byte with two hex digits: "{}""#,
            padded()
        ));
    }
    let mut bytes = [0u8; 6];
    for (byte, part) in bytes.iter_mut().zip(&parts) {
        *byte = u8::from_str_radix(part, 16).expect("checked hex");
    }
    Ok(bytes)
}

fn is_hex(s: &str) -> bool {
    s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// `"001a2b"` → `"00:1a:2b"`.
fn join_pairs(digits: &str, separator: char) -> String {
    digits
        .as_bytes()
        .chunks(2)
        .map(|pair| core::str::from_utf8(pair).expect("ASCII"))
        .collect::<Vec<_>>()
        .join(&separator.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn err(s: &str) -> String {
        parse_mac(s).expect_err(s)
    }

    #[test]
    fn mac_values() {
        let bytes = [0x00, 0x1a, 0x2b, 0x3c, 0x4d, 0x5e];
        assert_eq!(parse_mac("00:1a:2b:3c:4d:5e"), Ok(bytes));
        assert_eq!(parse_mac("00-1A-2B-3C-4D-5E"), Ok(bytes));
        assert_eq!(parse_mac("FF:FF:FF:FF:FF:FF"), Ok([0xff; 6]));
    }

    #[test]
    fn mac_errors() {
        let cases = [
            (
                "",
                r#"empty MAC address; write six hex bytes, like "00:1a:2b:3c:4d:5e""#,
            ),
            (
                "00:1a:2b-3c:4d:5e",
                r#"use one separator throughout: "00:1a:2b:3c:4d:5e""#,
            ),
            (
                "00-1a:2b-3c:4d:5e",
                r#"use one separator throughout: "00-1a-2b-3c-4d-5e""#,
            ),
            (
                "001a.2b3c.4d5e",
                r#"write the address as six bytes: "00:1a:2b:3c:4d:5e""#,
            ),
            (
                "0:1a:2b:3c:4d:5e",
                r#"write each byte with two hex digits: "00:1a:2b:3c:4d:5e""#,
            ),
            (
                "0-1A-2B-3C-4D-5E",
                r#"write each byte with two hex digits: "00-1a-2b-3c-4d-5e""#,
            ),
            ("001a2b3c4d5e", r#"add separators: "00:1a:2b:3c:4d:5e""#),
            ("00:1a:2b:3c:4d", "a MAC address has 6 bytes, found 5"),
            (
                "00:1a:2b:3c:4d:5e:6f:70",
                "a MAC address has 6 bytes, found 8",
            ),
            ("00:1a:2b:3c:4d:5g", "byte `5g` is not hex"),
            (" 00:1a:2b:3c:4d:5e", "remove the spaces around the literal"),
            ("00:1a:2b:3c:4d:5e ", "remove the spaces around the literal"),
            (
                "001a-2b3c-4d5e",
                r#"write the address as six bytes: "00-1a-2b-3c-4d-5e""#,
            ),
            (
                "001A:2B3C:4D5E",
                r#"write the address as six bytes: "00:1a:2b:3c:4d:5e""#,
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

    #[test]
    fn mac_round_trip() {
        let mut state = 48u64;
        for _ in 0..10_000 {
            let bytes: [u8; 6] = core::array::from_fn(|_| splitmix(&mut state) as u8);
            let colon = bytes.map(|b| format!("{b:02x}")).join(":");
            let dash = bytes.map(|b| format!("{b:02X}")).join("-");
            let mixed: String = colon
                .chars()
                .map(|c| {
                    if splitmix(&mut state).is_multiple_of(2) {
                        c.to_ascii_uppercase()
                    } else {
                        c
                    }
                })
                .collect();
            for text in [&colon, &dash, &mixed] {
                assert_eq!(parse_mac(text), Ok(bytes), "input {text:?}");
            }
        }
    }
}
