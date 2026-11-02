//! `oci::platform!`: OCI platforms, `os/arch[/variant]` with Go's names.
//!
//! The OS and architecture lists are Go's `GOOS` and `GOARCH` values (as
//! `go tool dist list` prints them); the fixture in `testdata/oci_platform.tsv`
//! pins both lists and checks alias suggestions against containerd.

use crate::suggest;

/// Go's `GOOS` values.
pub const OS: [&str; 15] = [
    "aix",
    "android",
    "darwin",
    "dragonfly",
    "freebsd",
    "illumos",
    "ios",
    "js",
    "linux",
    "netbsd",
    "openbsd",
    "plan9",
    "solaris",
    "wasip1",
    "windows",
];

/// Go's `GOARCH` values.
pub const ARCH: [&str; 14] = [
    "386", "amd64", "arm", "arm64", "loong64", "mips", "mips64", "mips64le", "mipsle", "ppc64",
    "ppc64le", "riscv64", "s390x", "wasm",
];

/// The variants an architecture takes, and how errors list them.
fn variants(arch: &str) -> Option<(&'static [&'static str], &'static str)> {
    match arch {
        "arm" => Some((&["v5", "v6", "v7", "v8"], "v5, v6, v7 or v8")),
        "arm64" => Some((
            &[
                "v8", "v8.1", "v8.2", "v8.3", "v8.4", "v8.5", "v8.6", "v8.7", "v8.8", "v8.9", "v9",
                "v9.1", "v9.2", "v9.3", "v9.4", "v9.5",
            ],
            "v8, v8.1-v8.9, v9 or v9.1-v9.5",
        )),
        "amd64" => Some((&["v1", "v2", "v3", "v4"], "v1, v2, v3 or v4")),
        _ => None,
    }
}

/// Common non-Go architecture names: the Go name and the variant they imply.
fn arch_alias(arch: &str) -> Option<(&'static str, Option<&'static str>)> {
    Some(match arch {
        "x86_64" | "x86-64" => ("amd64", None),
        "aarch64" => ("arm64", None),
        "armv7" | "armhf" => ("arm", Some("v7")),
        "armv6" | "armel" => ("arm", Some("v6")),
        "i386" | "i686" | "x86" => ("386", None),
        _ => return None,
    })
}

/// The platform with OS and architecture aliases replaced by Go's names, in
/// lower case, keeping a written variant only where the architecture takes it.
fn corrected(os: &str, arch: &str, variant: Option<&str>) -> String {
    let os = match os.to_lowercase().as_str() {
        "macos" | "osx" => "darwin".to_string(),
        other => other.to_string(),
    };
    let arch = arch.to_lowercase();
    let (arch, implied) = arch_alias(&arch).unwrap_or((arch.as_str(), None));
    // A written variant, with the "v" added if it was left out, where the arch takes it.
    let allowed = |v: &str| variants(arch).is_some_and(|(list, _)| list.contains(&v));
    let written = variant.map(str::to_lowercase).and_then(|v| {
        let prefixed = format!("v{v}");
        if allowed(&v) {
            Some(v)
        } else if allowed(&prefixed) {
            Some(prefixed)
        } else {
            None
        }
    });
    match written.as_deref().or(implied) {
        Some(v) => format!("{os}/{arch}/{v}"),
        None => format!("{os}/{arch}"),
    }
}

/// Parses `os/arch` or `os/arch/variant`.
pub fn parse_platform(s: &str) -> Result<(&str, &str, Option<&str>), String> {
    if s.is_empty() {
        return Err(r#"empty platform; write e.g. "linux/amd64" or "linux/arm64/v8""#.into());
    }
    if s.starts_with(char::is_whitespace) || s.ends_with(char::is_whitespace) {
        return Err("remove the spaces around the literal".into());
    }
    let parts: Vec<&str> = s.split('/').collect();
    match parts.len() {
        1 => return Err(r#"write the architecture too, like "linux/amd64""#.into()),
        2 | 3 => {}
        _ => return Err(r#"a platform is "os/arch" or "os/arch/variant""#.into()),
    }
    if parts.iter().any(|p| p.is_empty()) {
        return Err(format!(r#"empty part in "{s}""#));
    }
    let lower = s.to_lowercase();
    if lower != s && parse_platform(&lower).is_ok() {
        return Err(format!(r#"platforms are lower case: "{lower}""#));
    }
    let (os, arch, variant) = (parts[0], parts[1], parts.get(2).copied());
    if !OS.contains(&os) {
        if matches!(os.to_lowercase().as_str(), "macos" | "osx") {
            let suggestion = corrected(os, arch, variant);
            // Offer the whole platform only when it is valid; otherwise the
            // architecture has its own problem, reported once the OS is fixed.
            return Err(if parse_platform(&suggestion).is_ok() {
                format!(r#""{os}" is not a Go OS name; write "{suggestion}""#)
            } else {
                format!(r#""{os}" is not a Go OS name; use "darwin""#)
            });
        }
        return Err(match suggest::closest(os, OS) {
            Some(near) => format!(r#"unknown OS "{os}"; did you mean "{near}"?"#),
            None => format!(
                r#"unknown OS "{os}"; use a Go GOOS name like "linux", "windows" or "darwin""#
            ),
        });
    }
    if !ARCH.contains(&arch) {
        if arch_alias(&arch.to_lowercase()).is_some() {
            let suggestion = corrected(os, arch, variant);
            return Err(format!(
                r#""{arch}" is not a Go architecture name; write "{suggestion}""#
            ));
        }
        return Err(match suggest::closest(arch, ARCH) {
            Some(near) => format!(r#"unknown architecture "{arch}"; did you mean "{near}"?"#),
            None => format!(
                r#"unknown architecture "{arch}"; use a Go GOARCH name like "amd64" or "arm64""#
            ),
        });
    }
    if let Some(variant) = variant {
        let Some((allowed, listed)) = variants(arch) else {
            return Err(format!(r#"{arch} takes no variant; write "{os}/{arch}""#));
        };
        if !allowed.contains(&variant) {
            let prefixed = format!("v{variant}");
            if allowed.contains(&prefixed.as_str()) {
                return Err(format!(
                    r#"write the variant as "{prefixed}": "{os}/{arch}/{prefixed}""#
                ));
            }
            return Err(format!(
                r#"variant "{variant}" doesn't apply to {arch}; use {listed}"#
            ));
        }
    }
    Ok((os, arch, variant))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_values() {
        let cases = [
            ("linux/amd64", ("linux", "amd64", None)),
            ("linux/arm64/v8", ("linux", "arm64", Some("v8"))),
            ("linux/arm/v7", ("linux", "arm", Some("v7"))),
            ("windows/amd64", ("windows", "amd64", None)),
            ("darwin/arm64", ("darwin", "arm64", None)),
            ("linux/arm64/v9.5", ("linux", "arm64", Some("v9.5"))),
            ("linux/amd64/v3", ("linux", "amd64", Some("v3"))),
            ("wasip1/wasm", ("wasip1", "wasm", None)),
            ("linux/s390x", ("linux", "s390x", None)),
        ];
        for (input, expected) in cases {
            assert_eq!(parse_platform(input), Ok(expected), "input {input:?}");
        }
    }

    #[test]
    fn platform_errors() {
        let cases = [
            (
                "",
                r#"empty platform; write e.g. "linux/amd64" or "linux/arm64/v8""#,
            ),
            (" linux/amd64", "remove the spaces around the literal"),
            ("linux", r#"write the architecture too, like "linux/amd64""#),
            (
                "linux/arm/v7/x",
                r#"a platform is "os/arch" or "os/arch/variant""#,
            ),
            ("linux//amd64", r#"empty part in "linux//amd64""#),
            ("linux/amd64/", r#"empty part in "linux/amd64/""#),
            ("Linux/AMD64", r#"platforms are lower case: "linux/amd64""#),
            (
                "macos/arm64",
                r#""macos" is not a Go OS name; write "darwin/arm64""#,
            ),
            (
                "osx/arm64",
                r#""osx" is not a Go OS name; write "darwin/arm64""#,
            ),
            (
                "linux/x86_64",
                r#""x86_64" is not a Go architecture name; write "linux/amd64""#,
            ),
            (
                "linux/aarch64",
                r#""aarch64" is not a Go architecture name; write "linux/arm64""#,
            ),
            (
                "linux/armhf",
                r#""armhf" is not a Go architecture name; write "linux/arm/v7""#,
            ),
            (
                "linux/armel",
                r#""armel" is not a Go architecture name; write "linux/arm/v6""#,
            ),
            (
                "linux/i686",
                r#""i686" is not a Go architecture name; write "linux/386""#,
            ),
            ("lnux/amd64", r#"unknown OS "lnux"; did you mean "linux"?"#),
            (
                "beos/amd64",
                r#"unknown OS "beos"; use a Go GOOS name like "linux", "windows" or "darwin""#,
            ),
            (
                "linux/amd46",
                r#"unknown architecture "amd46"; did you mean "amd64"?"#,
            ),
            (
                "linux/vax",
                r#"unknown architecture "vax"; use a Go GOARCH name like "amd64" or "arm64""#,
            ),
            (
                "linux/amd64/v8",
                r#"variant "v8" doesn't apply to amd64; use v1, v2, v3 or v4"#,
            ),
            (
                "linux/arm/v4",
                r#"variant "v4" doesn't apply to arm; use v5, v6, v7 or v8"#,
            ),
            (
                "linux/arm64/v7",
                r#"variant "v7" doesn't apply to arm64; use v8, v8.1-v8.9, v9 or v9.1-v9.5"#,
            ),
            ("linux/386/v1", r#"386 takes no variant; write "linux/386""#),
            (
                "linux/arm/7",
                r#"write the variant as "v7": "linux/arm/v7""#,
            ),
            ("macos/vax", r#""macos" is not a Go OS name; use "darwin""#),
            (
                "macos/amd46",
                r#""macos" is not a Go OS name; use "darwin""#,
            ),
            (
                "MacOS/arm64",
                r#""MacOS" is not a Go OS name; write "darwin/arm64""#,
            ),
            (
                "linux/X86_64",
                r#""X86_64" is not a Go architecture name; write "linux/amd64""#,
            ),
            (
                "linux/ARMHF",
                r#""ARMHF" is not a Go architecture name; write "linux/arm/v7""#,
            ),
            (
                "linux/x86_64/3",
                r#""x86_64" is not a Go architecture name; write "linux/amd64/v3""#,
            ),
        ];
        for (input, message) in cases {
            assert_eq!(
                parse_platform(input),
                Err(message.to_string()),
                "input {input:?}"
            );
        }
    }

    #[test]
    fn platform_dist() {
        let fixture = include_str!("testdata/oci_platform.tsv");
        let pairs: Vec<&str> = fixture
            .lines()
            .filter_map(|l| l.strip_prefix("dist\t"))
            .collect();
        assert!(pairs.len() > 40, "only {} dist pairs", pairs.len());
        for pair in &pairs {
            let (os, arch) = pair.split_once('/').unwrap();
            assert!(OS.contains(&os), "{os} missing from OS");
            assert!(ARCH.contains(&arch), "{arch} missing from ARCH");
            assert_eq!(parse_platform(pair), Ok((os, arch, None)), "{pair}");
        }
        for os in OS {
            assert!(
                pairs.iter().any(|p| p.starts_with(&format!("{os}/"))),
                "{os} not in Go's list"
            );
        }
        for arch in ARCH {
            assert!(
                pairs.iter().any(|p| p.ends_with(&format!("/{arch}"))),
                "{arch} not in Go's list"
            );
        }
    }

    #[test]
    fn platform_containerd() {
        let fixture = include_str!("testdata/oci_platform.tsv");
        let mut compared = 0;
        for line in fixture.lines().filter_map(|l| l.strip_prefix("parse\t")) {
            let fields: Vec<&str> = line.split('\t').collect();
            let (input, verdict) = (fields[0], fields[1]);
            compared += 1;
            match parse_platform(input) {
                // containerd's parser has no "." in variants; the OCI image index
                // spec lists arm64's v8.1-v8.9 and v9.1-v9.5, and the spec keeps them.
                Ok((_, _, Some(variant))) if variant.contains('.') => {}
                Ok(_) => assert_eq!(verdict, "ok", "containerd rejects {input:?}, we accept it"),
                Err(message) => {
                    if let Some(start) = message.rfind("write \"") {
                        let fixed = message[start + 7..].trim_end_matches('"');
                        assert!(
                            parse_platform(fixed).is_ok(),
                            "{input:?}: the suggestion {fixed:?} is invalid"
                        );
                    }
                    // Where containerd rewrites an alias, our suggestion must be its result.
                    let normalized = fields.get(2).copied().unwrap_or("");
                    let alias = message.contains("is not a Go OS name")
                        || message.contains("is not a Go architecture name");
                    if alias && verdict == "ok" && normalized != input {
                        let ours = message.rsplit('"').nth(1).unwrap();
                        // containerd doesn't check variants and adds or drops default
                        // ones (arm becomes arm/v7, arm64/v8 becomes arm64), while we keep
                        // what is written. So compare os/arch, and the variant only where
                        // the alias itself implies one and none was written.
                        let os_arch = |p: &str| p.split('/').take(2).collect::<Vec<_>>().join("/");
                        let parts: Vec<&str> = input.split('/').collect();
                        let implies_variant = parts.len() == 2
                            && arch_alias(&parts[1].to_lowercase())
                                .is_some_and(|(_, v)| v.is_some());
                        let matches = if implies_variant {
                            ours == normalized
                        } else {
                            os_arch(ours) == os_arch(normalized)
                        };
                        assert!(
                            matches,
                            "{input:?}: containerd says {normalized}, we say: {message}"
                        );
                    }
                }
            }
        }
        assert!(compared >= 1000, "only {compared} parse lines");
    }
}
