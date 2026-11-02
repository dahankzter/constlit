//! `docker::port!`: Docker port mappings, as `docker run -p` and Compose take them.
//!
//! The grammar is `github.com/docker/go-connections/nat.ParsePortSpec`'s; the
//! fixture in `testdata/docker_port.tsv` pins it. Beyond Go, a few spellings it
//! quietly accepts are errors with a suggestion (upper-case protocols, port 0,
//! leading zeros, `+`, an unbracketed IPv6 address, an empty host port or protocol).

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// A parsed port mapping; ranges are inclusive `(start, end)`.
pub struct Port {
    pub host_ip: Option<IpAddr>,
    pub host: Option<(u16, u16)>,
    pub container: (u16, u16),
    pub protocol: &'static str,
}

/// Parses `[host_ip:][host_port:]container_port[/protocol]`.
pub fn parse_port(s: &str) -> Result<Port, String> {
    if s.is_empty() {
        return Err(
            r#"empty port mapping; write e.g. "8080:80" or "127.0.0.1:8080:80/tcp""#.into(),
        );
    }
    if s.starts_with(char::is_whitespace) || s.ends_with(char::is_whitespace) {
        return Err("remove the spaces around the literal".into());
    }
    if s.contains(char::is_whitespace) {
        let joined: String = s.split_whitespace().collect();
        return suggest("remove the spaces", &joined);
    }
    if s.contains('+') {
        return suggest("write the port as plain digits", &s.replace('+', ""));
    }
    if s.starts_with('/') {
        return Err("missing container port".into());
    }
    let (rest, protocol, suffix) = match s.rsplit_once('/') {
        Some((rest, "")) => {
            // Docker reads a bare "/" as tcp, so any other problem is reported for
            // the mapping without it, and "/udp" is only the example.
            parse_port(rest)?;
            let fixed = format!("{rest}/udp");
            return Err(format!(
                r#"missing protocol after "/"; remove the "/" or write e.g. "{fixed}""#
            ));
        }
        Some((rest, written)) => {
            let lower = written.to_ascii_lowercase();
            let protocol = match lower.as_str() {
                "tcp" => "tcp",
                "udp" => "udp",
                "sctp" => "sctp",
                _ => {
                    return Err(format!(
                        r#"unknown protocol "{written}"; use tcp, udp or sctp"#
                    ));
                }
            };
            if lower != written {
                return suggest("protocols are lower case", &format!("{rest}/{lower}"));
            }
            (rest, protocol, &s[rest.len()..])
        }
        None => (s, "tcp", ""),
    };
    // The host IP and the text before the ports, as written.
    let (host_ip, ip_prefix, ports) = if let Some(bracketed) = rest.strip_prefix('[') {
        if let Some(after) = bracketed.strip_prefix("]:") {
            return suggest(r#"remove the empty "[]""#, &format!("{after}{suffix}"));
        }
        let Some((ip, after)) = bracketed.split_once("]:") else {
            return Err(format!(
                r#""[{bracketed}" is not an IP address; write e.g. "[::1]:8080:80""#
            ));
        };
        if ip.parse::<Ipv4Addr>().is_ok() {
            return suggest(
                "IPv4 addresses don't take brackets",
                &format!("{ip}:{after}{suffix}"),
            );
        }
        if !after.contains(':') {
            return suggest(
                "add the host port, or leave it empty",
                &format!("[{ip}]::{after}{suffix}"),
            );
        }
        let Ok(ip) = ip.parse::<Ipv6Addr>() else {
            return Err(format!(r#""{ip}" is not an IPv6 address"#));
        };
        (
            Some(IpAddr::V6(ip)),
            &rest[..rest.len() - after.len()],
            after,
        )
    } else {
        let parts: Vec<&str> = rest.split(':').collect();
        if parts.len() == 2 && parts[0].parse::<Ipv4Addr>().is_ok() {
            return suggest(
                "add the host port, or leave it empty",
                &format!("{}::{}{suffix}", parts[0], parts[1]),
            );
        }
        if parts.len() <= 2 {
            (None, "", rest)
        } else {
            let ip_text = parts[..parts.len() - 2].join(":");
            let ports = &rest[ip_text.len() + 1..];
            if ip_text.is_empty() {
                let (host, container) = (parts[parts.len() - 2], parts[parts.len() - 1]);
                return if host.is_empty() {
                    suggest(
                        "remove the empty address and host port",
                        &format!("{container}{suffix}"),
                    )
                } else {
                    suggest(
                        "remove the empty address",
                        &format!("{host}:{container}{suffix}"),
                    )
                };
            }
            if parts.len() > 3 {
                if ip_text.parse::<Ipv6Addr>().is_ok() {
                    return suggest(
                        "put an IPv6 address in brackets",
                        &format!("[{ip_text}]:{ports}{suffix}"),
                    );
                }
                return Err(format!(
                    r#""{ip_text}" is not an IP address; port mappings take an IP, like "127.0.0.1""#
                ));
            }
            let Ok(ip) = ip_text.parse::<Ipv4Addr>() else {
                return Err(format!(
                    r#""{ip_text}" is not an IP address; port mappings take an IP, like "127.0.0.1""#
                ));
            };
            (Some(IpAddr::V4(ip)), &rest[..ip_text.len() + 1], ports)
        }
    };
    let (host_text, container_text) = match ports.split_once(':') {
        Some((host, container)) => (Some(host), container),
        None => (None, ports),
    };
    if container_text.is_empty() {
        return Err(if host_text.is_some() {
            r#"missing container port after ":""#.into()
        } else {
            "missing container port".into()
        });
    }
    if host_text == Some("") && host_ip.is_none() {
        return suggest(
            "remove the empty host port",
            &format!("{container_text}{suffix}"),
        );
    }
    let host = match host_text.filter(|h| !h.is_empty()) {
        Some(text) => Some(range(text)?),
        None => None,
    };
    let container = range(container_text)?;
    // Leading zeros and port 0: suggest the mapping written plainly.
    let plain = |r: (u16, u16)| {
        if r.0 == r.1 {
            r.0.to_string()
        } else {
            format!("{}-{}", r.0, r.1)
        }
    };
    let has_zeros = |t: &str| t.split('-').any(|n| n.len() > 1 && n.starts_with('0'));
    if host_text.is_some_and(has_zeros) || has_zeros(container_text) {
        let host_part = match host {
            Some(h) => format!("{}:", plain(h)),
            None if host_text.is_some() => ":".into(),
            None => String::new(),
        };
        return suggest(
            "write the port without leading zeros",
            &format!("{ip_prefix}{host_part}{}{suffix}", plain(container)),
        );
    }
    if host.is_some_and(|(a, b)| a == 0 || b == 0) {
        let without = if host_ip.is_some() {
            format!("{ip_prefix}:{container_text}{suffix}")
        } else {
            format!("{container_text}{suffix}")
        };
        return suggest(
            "port 0 isn't a port; leave the host port out for any free port",
            &without,
        );
    }
    if container.0 == 0 || container.1 == 0 {
        return Err("port 0 isn't a port".into());
    }
    if let Some((h1, h2)) = host {
        let (host_len, container_len) = (h2 - h1 + 1, container.1 - container.0 + 1);
        if container_len > 1 && host_len != container_len {
            let ports = |n: u16| {
                if n == 1 {
                    "1 port".to_string()
                } else {
                    format!("{n} ports")
                }
            };
            return Err(format!(
                "the host range {} has {} and the container range {} has {}; give both the same length, or one container port",
                host_text.unwrap_or_default(),
                ports(host_len),
                container_text,
                container_len
            ));
        }
    }
    Ok(Port {
        host_ip,
        host,
        container,
        protocol,
    })
}

/// `Err("<message>: \"<fixed>\"")` when the fixed literal is valid; otherwise the
/// fixed literal's own error, so a suggestion never names an invalid mapping.
fn suggest(message: &str, fixed: &str) -> Result<Port, String> {
    match parse_port(fixed) {
        Ok(_) => Err(format!(r#"{message}: "{fixed}""#)),
        Err(e) => Err(e),
    }
}

/// `"80"` or `"8000-8010"`.
fn range(text: &str) -> Result<(u16, u16), String> {
    let (start, end) = text.split_once('-').unwrap_or((text, text));
    if start.is_empty() || end.is_empty() {
        return Err(format!(r#"missing port number in "{text}""#));
    }
    let (a, b) = (number(start)?, number(end)?);
    if b < a {
        return Err(format!(
            r#"the range {text} ends before it starts; write "{b}-{a}""#
        ));
    }
    Ok((a, b))
}

fn number(text: &str) -> Result<u16, String> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!("port `{text}` is not a number"));
    }
    match text.parse::<u32>() {
        Ok(n) if n <= 65535 => Ok(n as u16),
        _ => Err(format!("port {text} is out of range (1-65535)")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::IpAddr;

    fn ok(s: &str) -> Port {
        parse_port(s).unwrap_or_else(|e| panic!("{s:?}: {e}"))
    }

    #[test]
    fn port_values() {
        let v4: IpAddr = "127.0.0.1".parse().unwrap();
        let v6: IpAddr = "::1".parse().unwrap();
        type Expected = (Option<IpAddr>, Option<(u16, u16)>, (u16, u16), &'static str);
        let cases: [(&str, Expected); 9] = [
            ("80", (None, None, (80, 80), "tcp")),
            ("8080:80", (None, Some((8080, 8080)), (80, 80), "tcp")),
            (
                "127.0.0.1:8080:80",
                (Some(v4), Some((8080, 8080)), (80, 80), "tcp"),
            ),
            ("127.0.0.1::80", (Some(v4), None, (80, 80), "tcp")),
            (
                "[::1]:8080:80",
                (Some(v6), Some((8080, 8080)), (80, 80), "tcp"),
            ),
            (
                "8000-8010:8000-8010",
                (None, Some((8000, 8010)), (8000, 8010), "tcp"),
            ),
            ("8000-8010:80", (None, Some((8000, 8010)), (80, 80), "tcp")),
            ("8080:80/udp", (None, Some((8080, 8080)), (80, 80), "udp")),
            ("8080:80/sctp", (None, Some((8080, 8080)), (80, 80), "sctp")),
        ];
        for (input, (ip, host, container, protocol)) in cases {
            let p = ok(input);
            assert_eq!(
                (p.host_ip, p.host, p.container, p.protocol),
                (ip, host, container, protocol),
                "{input}"
            );
        }
    }

    #[test]
    fn port_errors() {
        let cases = [
            (
                "",
                r#"empty port mapping; write e.g. "8080:80" or "127.0.0.1:8080:80/tcp""#,
            ),
            (" 80", "remove the spaces around the literal"),
            ("8080:80/TCP", r#"protocols are lower case: "8080:80/tcp""#),
            (
                "::1:8080:80",
                r#"put an IPv6 address in brackets: "[::1]:8080:80""#,
            ),
            (
                "0:80",
                r#"port 0 isn't a port; leave the host port out for any free port: "80""#,
            ),
            ("80:0", "port 0 isn't a port"),
            ("080:80", r#"write the port without leading zeros: "80:80""#),
            ("+80", r#"write the port as plain digits: "80""#),
            (
                "8080:80/",
                r#"missing protocol after "/"; remove the "/" or write e.g. "8080:80/udp""#,
            ),
            (":80", r#"remove the empty host port: "80""#),
            (
                "8000:80-90",
                "the host range 8000 has 1 port and the container range 80-90 has 11; give both the same length, or one container port",
            ),
            (
                "8080:80/icmp",
                r#"unknown protocol "icmp"; use tcp, udp or sctp"#,
            ),
            ("65536:80", "port 65536 is out of range (1-65535)"),
            (
                "90-80:90-80",
                r#"the range 90-80 ends before it starts; write "80-90""#,
            ),
            ("8080:", r#"missing container port after ":""#),
            (
                "localhost:8080:80",
                r#""localhost" is not an IP address; port mappings take an IP, like "127.0.0.1""#,
            ),
            (
                "a:b:c:d",
                r#""a:b" is not an IP address; port mappings take an IP, like "127.0.0.1""#,
            ),
            ("8080 :80", r#"remove the spaces: "8080:80""#),
            ("80x", "port `80x` is not a number"),
            (
                "[::1]:80",
                r#"add the host port, or leave it empty: "[::1]::80""#,
            ),
            (
                "127.0.0.1:80",
                r#"add the host port, or leave it empty: "127.0.0.1::80""#,
            ),
            (
                "080:80/",
                r#"write the port without leading zeros: "80:80""#,
            ),
            ("8080: 80/TCP", r#"protocols are lower case: "8080:80/tcp""#),
            ("80 x", "port `80x` is not a number"),
            ("::80", r#"remove the empty address and host port: "80""#),
            ("[]:80:80", r#"remove the empty "[]": "80:80""#),
            ("1-", r#"missing port number in "1-""#),
            ("/", "missing container port"),
            ("/udp", "missing container port"),
            (
                "[1.2.3.4]:80:80",
                r#"IPv4 addresses don't take brackets: "1.2.3.4:80:80""#,
            ),
        ];
        for (input, message) in cases {
            assert_eq!(
                parse_port(input).err().as_deref(),
                Some(message),
                "input {input:?}"
            );
        }
    }

    /// Our mapping as Go's `nat.ParsePortSpec` lists it: one entry per port of an
    /// equal-length range, or one entry for a host range with one container port.
    fn as_go(p: &Port) -> String {
        let ip = p.host_ip.map(|ip| ip.to_string()).unwrap_or_default();
        let (c1, c2) = p.container;
        let entries: Vec<String> = match p.host {
            None => (c1..=c2)
                .map(|c| format!("{ip}||{c}/{}", p.protocol))
                .collect(),
            Some((h1, h2)) if c1 == c2 && h1 != h2 => {
                vec![format!("{ip}|{h1}-{h2}|{c1}/{}", p.protocol)]
            }
            Some((h1, _)) => (0..=c2 - c1)
                .map(|i| format!("{ip}|{}|{}/{}", h1 + i, c1 + i, p.protocol))
                .collect(),
        };
        entries.join(" ")
    }

    pub(crate) fn suggestion(message: &str) -> Option<&str> {
        message
            .strip_suffix('"')?
            .rsplit_once('"')
            .map(|(_, s)| s)
            .filter(|s| !s.is_empty())
    }

    #[test]
    fn port_go_oracle() {
        let fixture = include_str!("testdata/docker_port.tsv");
        let (mut compared, mut accepted) = (0, 0);
        for line in fixture.lines().filter(|l| !l.starts_with('#')) {
            let fields: Vec<&str> = line.split('\t').collect();
            let input = fields[0];
            compared += 1;
            let ours = parse_port(input);
            if let Err(message) = &ours
                && let Some(fixed) = suggestion(message).filter(|_| message.contains(": \""))
            {
                assert!(
                    parse_port(fixed).is_ok(),
                    "{input:?}: suggestion {fixed:?} is invalid"
                );
            }
            match (fields[1], ours) {
                ("ok", Ok(p)) => {
                    accepted += 1;
                    assert_eq!(as_go(&p), fields[2], "input {input:?}");
                }
                ("err", Err(_)) => {}
                ("ok", Err(message)) => {
                    accepted += 1;
                    // A suggestion must keep the protocol Docker would use.
                    if let Some(fixed) = suggestion(&message).filter(|_| message.contains(": \""))
                        && let Ok(p) = parse_port(fixed)
                    {
                        let go_proto = fields[2]
                            .split(' ')
                            .next()
                            .and_then(|m| m.rsplit('/').next());
                        assert_eq!(
                            Some(p.protocol),
                            go_proto,
                            "{input:?}: suggestion {fixed:?}"
                        );
                    }
                    let tokens: Vec<&str> = input.split([':', '-', '/']).collect();
                    let allowed = if message.starts_with("protocols are lower case") {
                        input
                            .rsplit('/')
                            .next()
                            .is_some_and(|p| p != p.to_lowercase())
                    } else if message.starts_with("put an IPv6 address in brackets") {
                        !input.contains('[') && input.matches(':').count() > 2
                    } else if message.starts_with("port 0 isn't a port") {
                        tokens.contains(&"0")
                    } else if message.starts_with("write the port without leading zeros") {
                        tokens.iter().any(|t| t.len() > 1 && t.starts_with('0'))
                    } else if message.starts_with("write the port as plain digits") {
                        input.contains('+')
                    } else if message.starts_with("missing protocol after") {
                        input.ends_with('/')
                    } else if message.starts_with("remove the empty host port")
                        || message.starts_with("remove the empty address")
                    {
                        input.starts_with(':')
                    } else if message.starts_with(r#"remove the empty "[]""#) {
                        input.starts_with("[]")
                    } else if message.starts_with("IPv4 addresses don't take brackets") {
                        input.starts_with('[') && input.contains('.')
                    } else {
                        message == "remove the spaces around the literal" && input.trim() != input
                    };
                    assert!(allowed, "Go accepts {input:?}, we say: {message}");
                }
                (_, Ok(_)) => panic!("Go rejects {input:?}, we accept it"),
                _ => unreachable!(),
            }
        }
        assert!(compared >= 2000, "only {compared} fixture lines");
        assert!(accepted >= 800, "only {accepted} accepted fixture lines");
    }
}
