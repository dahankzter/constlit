//! `oci::image!`: container image references, normalized like Docker.
//!
//! The grammar and normalization are `github.com/distribution/reference`'s
//! (`ParseNormalizedNamed`, then `TagNameOnly`); the fixture in
//! `testdata/oci_image.tsv` pins them.

use crate::oci_digest::{Algorithm, parse_digest};

/// A parsed reference: normalized registry and repository, and the canonical string.
pub struct Image {
    pub registry: String,
    pub repository: String,
    pub tag: Option<String>,
    /// The algorithm, the decoded bytes and the digest as written.
    pub digest: Option<(Algorithm, Vec<u8>, String)>,
    pub canonical: String,
}

/// Parses an image reference: `[registry/]repository[:tag][@digest]`.
pub fn parse_image(s: &str) -> Result<Image, String> {
    if s.is_empty() {
        return Err(
            r#"empty image reference; write e.g. "nginx:1.27" or "ghcr.io/org/app:1.2""#.into(),
        );
    }
    if s.starts_with(char::is_whitespace) || s.ends_with(char::is_whitespace) {
        return Err("remove the spaces around the literal".into());
    }
    if s.contains(char::is_whitespace) {
        let joined: String = s.split_whitespace().collect();
        return Err(format!(r#"remove the spaces: "{joined}""#));
    }
    if let Some((scheme, rest)) = s.split_once("://") {
        return Err(if rest.is_empty() {
            format!(r#"remove the "{scheme}://""#)
        } else {
            format!(r#"remove the "{scheme}://": "{rest}""#)
        });
    }
    if s.matches('@').count() > 1 {
        return Err(r#"a reference has at most one "@""#.into());
    }
    if s.len() == 64 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
        return Err(
            r#"a 64-digit hex string can't be a repository name; put a digest after "@""#.into(),
        );
    }
    let (name_tag, digest) = match s.split_once('@') {
        Some((name_tag, written)) => {
            let bare_hex = written.bytes().all(|b| b.is_ascii_hexdigit());
            if bare_hex && matches!(written.len(), 64 | 128) {
                let algorithm = if written.len() == 64 {
                    "sha256"
                } else {
                    "sha512"
                };
                return Err(format!(
                    r#"add the algorithm after "@": "{name_tag}@{algorithm}:{}""#,
                    written.to_ascii_lowercase()
                ));
            }
            if !written.contains(':') {
                return Err(
                    r#""@" must be followed by a digest, like "@sha256:" and 64 hex digits"#.into(),
                );
            }
            let (algorithm, bytes) = parse_digest(written)?;
            (name_tag, Some((algorithm, bytes, written.to_string())))
        }
        None => (s, None),
    };
    let last_slash = name_tag.rfind('/');
    let (name, tag) = match name_tag.rfind(':') {
        Some(colon) if last_slash.is_none_or(|slash| colon > slash) => {
            (&name_tag[..colon], Some(&name_tag[colon + 1..]))
        }
        _ => (name_tag, None),
    };
    if let Some(tag) = tag {
        check_tag(tag)?;
    }
    // Where the registry ends and the repository starts, as Docker decides it.
    let (domain, remainder) = match name.split_once('/') {
        Some((first, rest))
            if first == "localhost"
                || first.contains(['.', ':'])
                || first != first.to_lowercase() =>
        {
            if !first.contains(['.', ':']) && first != "localhost" {
                // Docker would take "MyOrg/app" as a host named "MyOrg". Suggest the
                // lower-case name only when that is a valid reference.
                let lowered = format!("{}{}", name.to_lowercase(), &s[name.len()..]);
                let valid = parse_image(&lowered).is_ok();
                return Err(match (first.to_lowercase() == "localhost", valid) {
                    (true, true) => format!(r#"write "localhost" in lower case: "{lowered}""#),
                    (false, true) => format!(
                        r#""{first}" would be read as a registry host; Docker Hub names are lower case: "{lowered}""#
                    ),
                    _ => format!(
                        r#""{first}" would be read as a registry host; Docker Hub names are lower-case letters, digits and separators"#
                    ),
                });
            }
            (Some(first), rest)
        }
        _ => (None, name),
    };
    if let Some(domain) = domain
        && !valid_domain(domain)
    {
        return Err(format!(r#"invalid registry "{domain}""#));
    }
    let lower = remainder.to_lowercase();
    if lower != remainder && valid_repository(&lower) {
        let start = name.len() - remainder.len();
        return Err(format!(
            r#"repository names must be lower case; write "{}{lower}{}""#,
            &s[..start],
            &s[name.len()..]
        ));
    }
    if !valid_repository(remainder) {
        return Err(format!(
            r#"invalid repository name "{remainder}": each part starts and ends with a lower-case letter or digit, with ".", "_", "__" or "-" between"#
        ));
    }
    let registry = match domain {
        None | Some("index.docker.io") => "docker.io",
        Some(domain) => domain,
    };
    let repository = if registry == "docker.io" && !remainder.contains('/') {
        format!("library/{remainder}")
    } else {
        remainder.to_string()
    };
    if repository.len() > 255 {
        return Err(format!(
            "repository names are at most 255 characters; this one has {}",
            repository.len()
        ));
    }
    let mut canonical = format!("{registry}/{repository}");
    match (tag, &digest) {
        (Some(tag), _) => canonical = format!("{canonical}:{tag}"),
        (None, None) => canonical.push_str(":latest"),
        (None, Some(_)) => {}
    }
    if let Some((_, _, written)) = &digest {
        canonical = format!("{canonical}@{written}");
    }
    Ok(Image {
        registry: registry.to_string(),
        repository,
        tag: tag.map(str::to_string),
        digest,
        canonical,
    })
}

/// `[A-Za-z0-9_][A-Za-z0-9_.-]{0,127}`.
fn check_tag(tag: &str) -> Result<(), String> {
    if tag.is_empty() {
        return Err(r#"empty tag after ":"; remove the ":" or add a tag, like ":1.27""#.into());
    }
    let word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    let valid = word(tag.as_bytes()[0]) && tag.bytes().all(|b| word(b) || b == b'.' || b == b'-');
    if !valid {
        return Err(format!(
            r#"invalid tag "{tag}": tags start with a letter, digit or "_" and use letters, digits, "_", "." and "-""#
        ));
    }
    if tag.len() > 128 {
        return Err(format!(
            "tags are at most 128 characters; this one has {}",
            tag.len()
        ));
    }
    Ok(())
}

/// Path components `[a-z0-9]+((\.|_|__|-+)[a-z0-9]+)*`, joined by `/`.
fn valid_repository(path: &str) -> bool {
    path.split('/').all(|component| {
        let alnum = |b: &u8| b.is_ascii_lowercase() || b.is_ascii_digit();
        let bytes = component.as_bytes();
        let mut i = 0;
        loop {
            let start = i;
            while i < bytes.len() && alnum(&bytes[i]) {
                i += 1;
            }
            if i == start {
                return false;
            }
            if i == bytes.len() {
                return true;
            }
            let separator = &component[i..];
            i += if separator.starts_with("__") {
                2
            } else if separator.starts_with(['.', '_']) {
                1
            } else {
                separator.bytes().take_while(|&b| b == b'-').count()
            };
            if i == start || separator.is_empty() || !bytes.get(i).is_some_and(alnum) {
                return false;
            }
        }
    })
}

/// A domain name or bracketed IPv6 address, with an optional `:port`.
fn valid_domain(domain: &str) -> bool {
    let (host, port) = if let Some(rest) = domain.strip_prefix('[') {
        let Some((inside, after)) = rest.split_once(']') else {
            return false;
        };
        if inside.is_empty() || !inside.bytes().all(|b| b.is_ascii_hexdigit() || b == b':') {
            return false;
        }
        match after {
            "" => return true,
            _ => match after.strip_prefix(':') {
                Some(port) => ("", Some(port)),
                None => return false,
            },
        }
    } else {
        match domain.rsplit_once(':') {
            Some((host, port)) => (host, Some(port)),
            None => (domain, None),
        }
    };
    if let Some(port) = port
        && (port.is_empty() || !port.bytes().all(|b| b.is_ascii_digit()))
    {
        return false;
    }
    host.is_empty() && domain.starts_with('[')
        || host.split('.').all(|component| {
            let bytes = component.as_bytes();
            !bytes.is_empty()
                && bytes
                    .iter()
                    .all(|b| b.is_ascii_alphanumeric() || *b == b'-')
                && bytes[0].is_ascii_alphanumeric()
                && bytes[bytes.len() - 1].is_ascii_alphanumeric()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &str = "sha256:abababababababababababababababababababababababababababababababab";

    fn parts(s: &str) -> (String, String, Option<String>, String) {
        let image = parse_image(s).unwrap_or_else(|e| panic!("{s:?}: {e}"));
        (image.registry, image.repository, image.tag, image.canonical)
    }

    #[test]
    fn image_values() {
        let some = |t: &str| Some(t.to_string());
        let cases = [
            (
                "nginx",
                (
                    "docker.io",
                    "library/nginx",
                    None,
                    "docker.io/library/nginx:latest",
                ),
            ),
            (
                "nginx:1.27",
                (
                    "docker.io",
                    "library/nginx",
                    some("1.27"),
                    "docker.io/library/nginx:1.27",
                ),
            ),
            (
                "user/app",
                ("docker.io", "user/app", None, "docker.io/user/app:latest"),
            ),
            (
                "docker.io/nginx",
                (
                    "docker.io",
                    "library/nginx",
                    None,
                    "docker.io/library/nginx:latest",
                ),
            ),
            (
                "index.docker.io/nginx",
                (
                    "docker.io",
                    "library/nginx",
                    None,
                    "docker.io/library/nginx:latest",
                ),
            ),
            (
                "localhost/app",
                ("localhost", "app", None, "localhost/app:latest"),
            ),
            (
                "localhost:5000/app",
                ("localhost:5000", "app", None, "localhost:5000/app:latest"),
            ),
            (
                "registry.example.com:5000/app",
                (
                    "registry.example.com:5000",
                    "app",
                    None,
                    "registry.example.com:5000/app:latest",
                ),
            ),
            (
                "app:5000",
                (
                    "docker.io",
                    "library/app",
                    some("5000"),
                    "docker.io/library/app:5000",
                ),
            ),
            (
                "registry:5000/app:1.2",
                ("registry:5000", "app", some("1.2"), "registry:5000/app:1.2"),
            ),
            (
                "Registry.Example.com/app",
                (
                    "Registry.Example.com",
                    "app",
                    None,
                    "Registry.Example.com/app:latest",
                ),
            ),
            (
                "app:V1",
                (
                    "docker.io",
                    "library/app",
                    some("V1"),
                    "docker.io/library/app:V1",
                ),
            ),
            (
                "[::1]:5000/app",
                ("[::1]:5000", "app", None, "[::1]:5000/app:latest"),
            ),
        ];
        for (input, (registry, repository, tag, canonical)) in cases {
            let expected = (
                registry.to_string(),
                repository.to_string(),
                tag,
                canonical.to_string(),
            );
            assert_eq!(parts(input), expected, "input {input:?}");
        }
        let digested = parse_image(&format!("nginx@{D}")).unwrap();
        assert_eq!(digested.canonical, format!("docker.io/library/nginx@{D}"));
        assert_eq!(digested.tag, None);
        assert_eq!(digested.digest.as_ref().map(|d| d.2.as_str()), Some(D));
        let both = parse_image(&format!("ghcr.io/org/app:1.2@{D}")).unwrap();
        assert_eq!(both.canonical, format!("ghcr.io/org/app:1.2@{D}"));
    }

    #[test]
    fn image_errors() {
        let repo = r#"each part starts and ends with a lower-case letter or digit, with ".", "_", "__" or "-" between"#;
        let tag =
            r#"tags start with a letter, digit or "_" and use letters, digits, "_", "." and "-""#;
        let at = r#""@" must be followed by a digest, like "@sha256:" and 64 hex digits"#;
        let hex64 = "0123456789abcdef".repeat(4);
        let cases = [
            (String::new(), r#"empty image reference; write e.g. "nginx:1.27" or "ghcr.io/org/app:1.2""#.to_string()),
            (" nginx".into(), "remove the spaces around the literal".into()),
            ("nginx :1".into(), r#"remove the spaces: "nginx:1""#.into()),
            ("https://ghcr.io/org/app".into(), r#"remove the "https://": "ghcr.io/org/app""#.into()),
            (
                "ghcr.io/Org/App:1.2".into(),
                r#"repository names must be lower case; write "ghcr.io/org/app:1.2""#.into(),
            ),
            (
                "MyOrg/app".into(),
                r#""MyOrg" would be read as a registry host; Docker Hub names are lower case: "myorg/app""#.into(),
            ),
            ("org/app-".into(), format!(r#"invalid repository name "org/app-": {repo}"#)),
            ("org//app".into(), format!(r#"invalid repository name "org//app": {repo}"#)),
            ("org/_app".into(), format!(r#"invalid repository name "org/_app": {repo}"#)),
            ("a..b".into(), format!(r#"invalid repository name "a..b": {repo}"#)),
            ("exa_mple.com/app".into(), r#"invalid registry "exa_mple.com""#.into()),
            ("host:x/app".into(), r#"invalid registry "host:x""#.into()),
            ("nginx:".into(), r#"empty tag after ":"; remove the ":" or add a tag, like ":1.27""#.into()),
            ("nginx:-x".into(), format!(r#"invalid tag "-x": {tag}"#)),
            ("nginx:a+b".into(), format!(r#"invalid tag "a+b": {tag}"#)),
            (
                format!("nginx:{}", "t".repeat(129)),
                "tags are at most 128 characters; this one has 129".into(),
            ),
            ("nginx@".into(), at.into()),
            ("nginx@latest".into(), at.into()),
            (
                format!("nginx@sha256:{}", "a".repeat(63)),
                "a sha256 digest has 64 hex digits, found 63".into(),
            ),
            (
                hex64,
                r#"a 64-digit hex string can't be a repository name; put a digest after "@""#.into(),
            ),
            (
                format!("x/{}", "a".repeat(254)),
                "repository names are at most 255 characters; this one has 256".into(),
            ),
            (
                "MyOrg/App".into(),
                r#""MyOrg" would be read as a registry host; Docker Hub names are lower case: "myorg/app""#.into(),
            ),
            (
                "ÄÖ/app".into(),
                r#""ÄÖ" would be read as a registry host; Docker Hub names are lower-case letters, digits and separators"#.into(),
            ),
            (
                "LocalHost/app".into(),
                r#"write "localhost" in lower case: "localhost/app""#.into(),
            ),
            ("x://".into(), r#"remove the "x://""#.into()),
            (
                format!("nginx@{}", "ab".repeat(32)),
                format!(r#"add the algorithm after "@": "nginx@sha256:{}""#, "ab".repeat(32)),
            ),
            (
                format!("a@b@sha256:{}", "ab".repeat(32)),
                r#"a reference has at most one "@""#.into(),
            ),
        ];
        for (input, message) in cases {
            assert_eq!(parse_image(&input).err(), Some(message), "input {input:?}");
        }
    }

    /// A quoted literal a message suggests, after "write", "lower case:" and the like.
    pub(crate) fn suggestion(message: &str) -> Option<&str> {
        let markers = [
            "write \"",
            "lower case: \"",
            "are lower case: \"",
            "after \"@\": \"",
            "algorithm: \"",
        ];
        markers.iter().find_map(|m| {
            let start = message.rfind(m)? + m.len();
            message[start..].strip_suffix('"')
        })
    }

    fn assert_suggestion_parses(input: &str, message: &str) {
        if let Some(fixed) = suggestion(message) {
            assert!(
                parse_image(fixed).is_ok(),
                "{input:?}: the suggestion {fixed:?} is invalid: {:?}",
                parse_image(fixed).err()
            );
        }
    }

    #[test]
    fn image_go_oracle() {
        let fixture = include_str!("testdata/oci_image.tsv");
        let (mut compared, mut accepted) = (0, 0);
        for line in fixture.lines().filter(|l| !l.starts_with('#')) {
            let fields: Vec<&str> = line.split('\t').collect();
            let input = fields[0];
            compared += 1;
            match (fields[1], parse_image(input)) {
                ("ok", Ok(image)) => {
                    accepted += 1;
                    let dash = |x: Option<&str>| x.unwrap_or("-").to_string();
                    let ours = [
                        image.canonical.clone(),
                        image.registry.clone(),
                        image.repository.clone(),
                        dash(image.tag.as_deref()),
                        dash(image.digest.as_ref().map(|d| d.2.as_str())),
                    ];
                    assert_eq!(ours.to_vec(), fields[2..7].to_vec(), "input {input:?}");
                }
                ("err", Err(message)) => assert_suggestion_parses(input, &message),
                ("ok", Err(message)) => {
                    assert_suggestion_parses(input, &message);
                    accepted += 1;
                    let first = input.split('/').next().unwrap_or("");
                    let allowed = if message.contains("would be read as a registry host") {
                        input.contains('/')
                            && first != first.to_lowercase()
                            && !first.contains(['.', ':'])
                    } else if message.starts_with(r#"unsupported algorithm "sha384""#) {
                        input.contains("@sha384:")
                    } else if message.starts_with("invalid registry") {
                        // Go reads a host it can't parse as part of the path,
                        // leaving the reference with no registry at all.
                        fields[3].is_empty()
                    } else {
                        message.starts_with("remove the spaces") && input.trim() != input
                    };
                    assert!(allowed, "Go accepts {input:?}, we say: {message}");
                }
                (_, Ok(_)) => panic!("Go rejects {input:?}, we accept it"),
                _ => unreachable!(),
            }
        }
        assert!(compared >= 2000, "only {compared} fixture lines");
        assert!(accepted >= 1000, "only {accepted} accepted fixture lines");
    }
}
