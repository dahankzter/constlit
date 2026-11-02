//! `linux::cap!` and `linux::caps!`: Linux capability names.

use crate::suggest;

/// Capability names by number (stable kernel ABI). The runtime crate keeps its own
/// copy for `Capability::name`; each copy is tested against the same list.
const NAMES: [&str; 41] = [
    "CAP_CHOWN",
    "CAP_DAC_OVERRIDE",
    "CAP_DAC_READ_SEARCH",
    "CAP_FOWNER",
    "CAP_FSETID",
    "CAP_KILL",
    "CAP_SETGID",
    "CAP_SETUID",
    "CAP_SETPCAP",
    "CAP_LINUX_IMMUTABLE",
    "CAP_NET_BIND_SERVICE",
    "CAP_NET_BROADCAST",
    "CAP_NET_ADMIN",
    "CAP_NET_RAW",
    "CAP_IPC_LOCK",
    "CAP_IPC_OWNER",
    "CAP_SYS_MODULE",
    "CAP_SYS_RAWIO",
    "CAP_SYS_CHROOT",
    "CAP_SYS_PTRACE",
    "CAP_SYS_PACCT",
    "CAP_SYS_ADMIN",
    "CAP_SYS_BOOT",
    "CAP_SYS_NICE",
    "CAP_SYS_RESOURCE",
    "CAP_SYS_TIME",
    "CAP_SYS_TTY_CONFIG",
    "CAP_MKNOD",
    "CAP_LEASE",
    "CAP_AUDIT_WRITE",
    "CAP_AUDIT_CONTROL",
    "CAP_SETFCAP",
    "CAP_MAC_OVERRIDE",
    "CAP_MAC_ADMIN",
    "CAP_SYSLOG",
    "CAP_WAKE_ALARM",
    "CAP_BLOCK_SUSPEND",
    "CAP_AUDIT_READ",
    "CAP_PERFMON",
    "CAP_BPF",
    "CAP_CHECKPOINT_RESTORE",
];

/// Parses a capability name (`"CAP_NET_ADMIN"` or `"NET_ADMIN"`) into its number.
pub fn parse_cap(s: &str) -> Result<u8, String> {
    if s.is_empty() {
        return Err(r#"empty capability; write a name like "CAP_NET_ADMIN""#.into());
    }
    if s.starts_with(char::is_whitespace) || s.ends_with(char::is_whitespace) {
        return Err("remove the spaces around the literal".into());
    }
    if s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(match s.parse::<usize>().ok().and_then(|n| NAMES.get(n)) {
            Some(name) => format!(r#"write the capability name: "{name}""#),
            None => r#"write the capability name, like "CAP_NET_ADMIN""#.into(),
        });
    }
    if s.contains(',') {
        return Err("cap! takes one capability; use caps! for a list".into());
    }
    // Checks below run on the upper-cased name, so "all" and lower-case typos get
    // the same help as their upper-case spellings.
    let upper = s.to_ascii_uppercase();
    if upper != s && lookup(&upper).is_some() {
        return Err(format!(r#"capability names are upper case: "{upper}""#));
    }
    let name = upper.strip_prefix("CAP_").unwrap_or(&upper);
    if name == "ALL" {
        return Err(r#""ALL" depends on the kernel; list the capabilities you need"#.into());
    }
    if let Some(number) = lookup(s) {
        return Ok(number);
    }
    // Compared without "CAP_", so the shared prefix doesn't make far-off names look close.
    let candidates = NAMES.iter().map(|n| &n[4..]);
    Err(match suggest::closest(name, candidates) {
        Some(near) => format!(r#"unknown capability "{s}"; did you mean "CAP_{near}"?"#),
        None => format!(r#"unknown capability "{s}""#),
    })
}

/// Parses a comma-separated list of capability names into a bitmask by number.
pub fn parse_caps(s: &str) -> Result<u64, String> {
    if s.is_empty() {
        return Err("empty capability set; use CapSet::EMPTY".into());
    }
    if s.starts_with(char::is_whitespace) || s.ends_with(char::is_whitespace) {
        return Err("remove the spaces around the literal".into());
    }
    if s.starts_with(',') {
        return Err(r#"remove the leading ",""#.into());
    }
    if s.ends_with(',') {
        return Err(r#"remove the trailing ",""#.into());
    }
    let mut bits = 0u64;
    for (i, part) in s.split(',').enumerate() {
        let name = if i > 0 {
            part.strip_prefix(' ').unwrap_or(part)
        } else {
            part
        };
        if name.is_empty() {
            return Err("empty name between commas".into());
        }
        if name.starts_with(char::is_whitespace) || name.ends_with(char::is_whitespace) {
            return Err(r#"separate names with "," or ", ""#.into());
        }
        let number = parse_cap(name)?;
        if bits & (1 << number) != 0 {
            return Err(format!("{} is listed twice", NAMES[usize::from(number)]));
        }
        bits |= 1 << number;
    }
    Ok(bits)
}

/// The number of `s`, with or without `CAP_`.
fn lookup(s: &str) -> Option<u8> {
    let name = s.strip_prefix("CAP_").unwrap_or(s);
    NAMES.iter().position(|n| &n[4..] == name).map(|i| i as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cap_values() {
        assert_eq!(parse_cap("CAP_NET_ADMIN"), Ok(12));
        assert_eq!(parse_cap("NET_ADMIN"), Ok(12));
        assert_eq!(parse_cap("CAP_CHOWN"), Ok(0));
        assert_eq!(parse_cap("CAP_CHECKPOINT_RESTORE"), Ok(40));
    }

    /// The plan's capability table, numbers 0-40.
    const PLAN: [&str; 41] = [
        "CAP_CHOWN",
        "CAP_DAC_OVERRIDE",
        "CAP_DAC_READ_SEARCH",
        "CAP_FOWNER",
        "CAP_FSETID",
        "CAP_KILL",
        "CAP_SETGID",
        "CAP_SETUID",
        "CAP_SETPCAP",
        "CAP_LINUX_IMMUTABLE",
        "CAP_NET_BIND_SERVICE",
        "CAP_NET_BROADCAST",
        "CAP_NET_ADMIN",
        "CAP_NET_RAW",
        "CAP_IPC_LOCK",
        "CAP_IPC_OWNER",
        "CAP_SYS_MODULE",
        "CAP_SYS_RAWIO",
        "CAP_SYS_CHROOT",
        "CAP_SYS_PTRACE",
        "CAP_SYS_PACCT",
        "CAP_SYS_ADMIN",
        "CAP_SYS_BOOT",
        "CAP_SYS_NICE",
        "CAP_SYS_RESOURCE",
        "CAP_SYS_TIME",
        "CAP_SYS_TTY_CONFIG",
        "CAP_MKNOD",
        "CAP_LEASE",
        "CAP_AUDIT_WRITE",
        "CAP_AUDIT_CONTROL",
        "CAP_SETFCAP",
        "CAP_MAC_OVERRIDE",
        "CAP_MAC_ADMIN",
        "CAP_SYSLOG",
        "CAP_WAKE_ALARM",
        "CAP_BLOCK_SUSPEND",
        "CAP_AUDIT_READ",
        "CAP_PERFMON",
        "CAP_BPF",
        "CAP_CHECKPOINT_RESTORE",
    ];

    #[test]
    fn cap_table() {
        for (number, name) in PLAN.iter().enumerate() {
            assert_eq!(parse_cap(name), Ok(number as u8), "{name}");
            assert_eq!(parse_cap(&name[4..]), Ok(number as u8), "{name}");
        }
    }

    #[test]
    fn cap_errors() {
        let cases = [
            ("", r#"empty capability; write a name like "CAP_NET_ADMIN""#),
            (
                "cap_net_admin",
                r#"capability names are upper case: "CAP_NET_ADMIN""#,
            ),
            (
                "net_admin",
                r#"capability names are upper case: "NET_ADMIN""#,
            ),
            (
                "ALL",
                r#""ALL" depends on the kernel; list the capabilities you need"#,
            ),
            (
                "CAP_ALL",
                r#""ALL" depends on the kernel; list the capabilities you need"#,
            ),
            ("12", r#"write the capability name: "CAP_NET_ADMIN""#),
            ("0", r#"write the capability name: "CAP_CHOWN""#),
            ("41", r#"write the capability name, like "CAP_NET_ADMIN""#),
            (
                "CAP_NET_ADMN",
                r#"unknown capability "CAP_NET_ADMN"; did you mean "CAP_NET_ADMIN"?"#,
            ),
            (
                "NET_ADMN",
                r#"unknown capability "NET_ADMN"; did you mean "CAP_NET_ADMIN"?"#,
            ),
            ("CAP_FOO", r#"unknown capability "CAP_FOO""#),
            (" CAP_KILL", "remove the spaces around the literal"),
            (
                "all",
                r#""ALL" depends on the kernel; list the capabilities you need"#,
            ),
            (
                "cap_all",
                r#""ALL" depends on the kernel; list the capabilities you need"#,
            ),
            (
                "cap_net_admn",
                r#"unknown capability "cap_net_admn"; did you mean "CAP_NET_ADMIN"?"#,
            ),
            (
                "CAP_KILL,CAP_CHOWN",
                "cap! takes one capability; use caps! for a list",
            ),
        ];
        for (input, message) in cases {
            assert_eq!(
                parse_cap(input).expect_err(input),
                message,
                "input {input:?}"
            );
        }
    }

    #[test]
    fn caps_values() {
        assert_eq!(
            parse_caps("CAP_NET_ADMIN, CAP_SYS_TIME"),
            Ok((1 << 12) | (1 << 25))
        );
        assert_eq!(parse_caps("CHOWN,KILL"), Ok(0b100001));
        assert_eq!(parse_caps("CAP_CHECKPOINT_RESTORE"), Ok(1 << 40));
    }

    #[test]
    fn caps_errors() {
        let cases = [
            ("", "empty capability set; use CapSet::EMPTY"),
            ("CAP_KILL,", r#"remove the trailing ",""#),
            ("CAP_KILL,,CAP_CHOWN", "empty name between commas"),
            ("CAP_KILL ,CAP_CHOWN", r#"separate names with "," or ", ""#),
            ("CAP_KILL,  CAP_CHOWN", r#"separate names with "," or ", ""#),
            ("NET_ADMIN, CAP_NET_ADMIN", "CAP_NET_ADMIN is listed twice"),
            (
                "CAP_KILL, cap_chown",
                r#"capability names are upper case: "CAP_CHOWN""#,
            ),
            (
                "CAP_KILL, ALL",
                r#""ALL" depends on the kernel; list the capabilities you need"#,
            ),
            (" CAP_KILL", "remove the spaces around the literal"),
            (",CAP_KILL", r#"remove the leading ",""#),
        ];
        for (input, message) in cases {
            assert_eq!(
                parse_caps(input).expect_err(input),
                message,
                "input {input:?}"
            );
        }
    }
}
