//! `oci::digest!`: OCI content digests, `sha256` or `sha512`.
//!
//! The rules are go-digest's (`digest.Parse`), restricted to the two algorithms
//! the OCI image spec registers.

use proc_macro2::TokenStream;
use quote::quote;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Algorithm {
    Sha256,
    Sha512,
}

impl Algorithm {
    fn name(self) -> &'static str {
        match self {
            Algorithm::Sha256 => "sha256",
            Algorithm::Sha512 => "sha512",
        }
    }

    fn hex_len(self) -> usize {
        match self {
            Algorithm::Sha256 => 64,
            Algorithm::Sha512 => 128,
        }
    }
}

const LIKE: &str = r#"like "sha256:" followed by 64 hex digits"#;

/// Parses `<algorithm>:<hex>` into the algorithm and the decoded bytes.
pub fn parse_digest(s: &str) -> Result<(Algorithm, Vec<u8>), String> {
    if s.is_empty() {
        return Err(r#"empty digest; write e.g. "sha256:" followed by 64 hex digits"#.into());
    }
    if s.starts_with(char::is_whitespace) || s.ends_with(char::is_whitespace) {
        return Err("remove the spaces around the literal".into());
    }
    let Some((name, hex)) = s.split_once(':').filter(|(name, _)| !name.is_empty()) else {
        let bare = s.bytes().all(|b| b.is_ascii_hexdigit());
        return Err(match (bare, s.len()) {
            (true, 64) => format!(r#"add the algorithm: "sha256:{}""#, s.to_ascii_lowercase()),
            (true, 128) => format!(r#"add the algorithm: "sha512:{}""#, s.to_ascii_lowercase()),
            _ => format!(r#"write the algorithm, ":" and the hex digits, {LIKE}"#),
        });
    };
    let algorithm = match name.to_ascii_lowercase().as_str() {
        lower @ ("sha256" | "sha512") if lower != name => {
            return Err(format!(r#"write the algorithm in lower case: "{lower}""#));
        }
        "sha256" => Algorithm::Sha256,
        "sha512" => Algorithm::Sha512,
        _ => {
            return Err(format!(
                r#"unsupported algorithm "{name}"; OCI images use "sha256" or "sha512""#
            ));
        }
    };
    if let Some(c) = hex.chars().find(|c| !c.is_ascii_hexdigit()) {
        return Err(format!("digit `{c}` is not hex"));
    }
    if hex.len() != algorithm.hex_len() {
        return Err(format!(
            "a {} digest has {} hex digits, found {}",
            algorithm.name(),
            algorithm.hex_len(),
            hex.len()
        ));
    }
    if hex.bytes().any(|b| b.is_ascii_uppercase()) {
        return Err(format!(
            r#"digests use lower-case hex: "{}:{}""#,
            algorithm.name(),
            hex.to_ascii_lowercase()
        ));
    }
    let bytes = hex
        .as_bytes()
        .chunks(2)
        .map(|pair| {
            u8::from_str_radix(core::str::from_utf8(pair).expect("ASCII"), 16).expect("hex")
        })
        .collect();
    Ok((algorithm, bytes))
}

/// The expression building a `constlit::oci::Digest` with these bytes.
pub fn digest_tokens(algorithm: Algorithm, bytes: &[u8]) -> TokenStream {
    match algorithm {
        Algorithm::Sha256 => quote!(::constlit::oci::Digest::__sha256([#(#bytes),*])),
        Algorithm::Sha512 => quote!(::constlit::oci::Digest::__sha512([#(#bytes),*])),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const H64: &str = "abababababababababababababababababababababababababababababababab";

    fn hex(bytes: impl IntoIterator<Item = u8>) -> String {
        bytes.into_iter().map(|b| format!("{b:02x}")).collect()
    }

    #[test]
    fn digest_values() {
        assert_eq!(
            parse_digest(&format!("sha256:{}", hex([0; 32]))),
            Ok((Algorithm::Sha256, vec![0; 32]))
        );
        assert_eq!(
            parse_digest(&format!("sha512:{}", hex(1..=64))),
            Ok((Algorithm::Sha512, (1..=64).collect()))
        );
        assert_eq!(
            parse_digest(&format!("sha256:{H64}")),
            Ok((Algorithm::Sha256, vec![0xab; 32]))
        );
    }

    #[test]
    fn digest_errors() {
        let h128 = "ef".repeat(64);
        let cases = [
            (
                String::new(),
                r#"empty digest; write e.g. "sha256:" followed by 64 hex digits"#.to_string(),
            ),
            (
                format!(" sha256:{H64}"),
                "remove the spaces around the literal".into(),
            ),
            (H64.into(), format!(r#"add the algorithm: "sha256:{H64}""#)),
            (h128.clone(), format!(r#"add the algorithm: "sha512:{h128}""#)),
            (
                "abc".into(),
                r#"write the algorithm, ":" and the hex digits, like "sha256:" followed by 64 hex digits"#
                    .into(),
            ),
            (
                format!("sha384:{}", "a".repeat(96)),
                r#"unsupported algorithm "sha384"; OCI images use "sha256" or "sha512""#.into(),
            ),
            (
                format!("md5:{}", "a".repeat(32)),
                r#"unsupported algorithm "md5"; OCI images use "sha256" or "sha512""#.into(),
            ),
            (
                format!("SHA256:{H64}"),
                r#"write the algorithm in lower case: "sha256""#.into(),
            ),
            (
                format!("sha256:{}", &H64[..63]),
                "a sha256 digest has 64 hex digits, found 63".into(),
            ),
            (
                format!("sha512:{}", &h128[..127]),
                "a sha512 digest has 128 hex digits, found 127".into(),
            ),
            (
                format!("sha256:{}", H64.to_uppercase()),
                format!(r#"digests use lower-case hex: "sha256:{H64}""#),
            ),
            (
                format!("sha256:{}g", &H64[..63]),
                "digit `g` is not hex".into(),
            ),
            (
                "AB".repeat(32),
                format!(r#"add the algorithm: "sha256:{}""#, "ab".repeat(32)),
            ),
        ];
        for (input, message) in cases {
            assert_eq!(parse_digest(&input), Err(message), "input {input:?}");
        }
    }

    #[test]
    fn digest_go_oracle() {
        let fixture = include_str!("testdata/oci_digest.tsv");
        let (mut compared, mut accepted) = (0, 0);
        for line in fixture.lines().filter(|l| !l.starts_with('#')) {
            let (input, verdict) = line.split_once('\t').unwrap();
            compared += 1;
            accepted += usize::from(verdict == "ok");
            match (verdict, parse_digest(input)) {
                ("ok", Ok(_)) => {}
                ("err", Err(message)) => {
                    if let Some(start) = message.rfind(": \"") {
                        let fixed = message[start + 3..].trim_end_matches('"');
                        if fixed.contains(':') {
                            assert!(
                                parse_digest(fixed).is_ok(),
                                "{input:?}: suggestion {fixed:?}"
                            );
                        }
                    }
                }
                ("ok", Err(message)) => assert!(
                    message.starts_with(r#"unsupported algorithm "sha384""#)
                        && input.starts_with("sha384:"),
                    "Go accepts {input:?}, we say: {message}"
                ),
                (_, Ok(_)) => panic!("Go rejects {input:?}, we accept it"),
                _ => unreachable!(),
            }
        }
        assert!(compared >= 1500, "only {compared} fixture lines");
        assert!(accepted >= 400, "only {accepted} accepted fixture lines");
    }
}
