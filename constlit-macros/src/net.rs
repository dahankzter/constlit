//! `ip!` and `addr!`: IP and socket addresses.

use proc_macro2::TokenStream;
use quote::quote;
use std::net::{Ipv4Addr, Ipv6Addr};

/// A parsed IP address, as octets or segments.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ip {
    V4([u8; 4]),
    V6([u16; 8]),
}

impl Ip {
    /// A `core::net::Ipv4Addr` or `Ipv6Addr` constructor call.
    pub fn to_tokens(self) -> TokenStream {
        match self {
            Ip::V4([a, b, c, d]) => quote!(::core::net::Ipv4Addr::new(#a, #b, #c, #d)),
            Ip::V6(segments) => quote!(::core::net::Ipv6Addr::new(#(#segments),*)),
        }
    }
}

/// For dotted IPv4 with leading zeros: the first such part, and `s` without them
/// when that is a valid address.
fn without_leading_zeros(s: &str) -> Option<(&str, Option<String>)> {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 4
        || !parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
    {
        return None;
    }
    let first = parts.iter().find(|p| p.len() > 1 && p.starts_with('0'))?;
    let fixed: Vec<&str> = parts
        .iter()
        .map(|p| match p.trim_start_matches('0') {
            "" => "0",
            trimmed => trimmed,
        })
        .collect();
    let fixed = fixed.join(".");
    let valid = fixed.parse::<Ipv4Addr>().is_ok();
    Some((first, valid.then_some(fixed)))
}

/// Parses an IPv4 or IPv6 address literal.
pub fn parse_ip(s: &str) -> Result<Ip, String> {
    if let Some(zone) = s.find('%').map(|i| &s[i..])
        && s[..s.len() - zone.len()].parse::<Ipv6Addr>().is_ok()
    {
        return Err(format!(
            "zone suffixes like {zone} can't be stored in Ipv6Addr; remove it"
        ));
    }
    if let Ok(v4) = s.parse::<Ipv4Addr>() {
        return Ok(Ip::V4(v4.octets()));
    }
    if let Ok(v6) = s.parse::<Ipv6Addr>() {
        return Ok(Ip::V6(v6.segments()));
    }
    if let Some((part, fixed)) = without_leading_zeros(s) {
        let problem =
            format!("IPv4 parts can't have leading zeros (is \"{part}\" decimal or octal?)");
        return Err(match fixed {
            Some(fixed) => format!("{problem}; write {fixed}"),
            None => problem,
        });
    }
    Err(format!("`{s}` is not an IPv4 or IPv6 address"))
}

fn is_hostname(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.')
        && s.bytes().any(|b| b.is_ascii_alphabetic())
}

fn port(port: &str, example_host: &str) -> Result<u16, String> {
    if port.is_empty() {
        return Err(format!("missing port; write e.g. \"{example_host}:8080\""));
    }
    if !port.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!("`{port}` is not a port number"));
    }
    port.parse::<u16>()
        .map_err(|_| format!("port {port} is out of range (0-65535)"))
}

/// Parses a socket address literal: `"ipv4:port"` or `"[ipv6]:port"`.
pub fn parse_socket_addr(s: &str) -> Result<(Ip, u16), String> {
    if let Some(bracketed) = s.strip_prefix('[') {
        let Some((inside, after)) = bracketed.split_once(']') else {
            return Err("missing `]` after the IPv6 address".into());
        };
        let ip = parse_ip(inside)?;
        if let Ip::V4(_) = ip {
            let port = after.strip_prefix(':').unwrap_or(after);
            return Err(format!(
                "brackets are only for IPv6 addresses; write \"{inside}:{port}\""
            ));
        }
        let example = format!("[{inside}]");
        return match after.strip_prefix(':') {
            Some(p) => Ok((ip, port(p, &example)?)),
            None if after.is_empty() => Err(format!("missing port; write e.g. \"{example}:8080\"")),
            None => Err(format!(
                "unexpected `{after}` after the address; write e.g. \"{example}:8080\""
            )),
        };
    }
    if s.matches(':').count() > 1 {
        return Err("IPv6 socket addresses need brackets, like \"[::1]:443\"".into());
    }
    let (host, port_text) = match s.split_once(':') {
        Some((host, port_text)) => (host, Some(port_text)),
        None => (s, None),
    };
    let ip = match parse_ip(host) {
        Ok(ip) => ip,
        Err(_) if is_hostname(host) => {
            return Err(format!(
                "`{host}` is a hostname; addr! needs an IP address (looking up a name isn't a literal)"
            ));
        }
        Err(error) => return Err(error),
    };
    Ok((ip, port(port_text.unwrap_or(""), host)?))
}

/// Parses a network literal `"addr/prefix"`; the address must have no host bits set.
pub fn parse_cidr(s: &str) -> Result<(Ip, u8), String> {
    let Some((addr, prefix)) = s.split_once('/') else {
        return Err("missing /prefix; write e.g. \"10.0.0.0/8\"".into());
    };
    let ip = parse_ip(addr)?;
    if prefix.is_empty() || !prefix.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!(
            "`/{prefix}` is not a prefix length; write e.g. \"10.0.0.0/8\""
        ));
    }
    let (family, max, bits) = match ip {
        Ip::V4(octets) => ("IPv4", 32, u128::from(u32::from_be_bytes(octets))),
        Ip::V6(segments) => ("IPv6", 128, Ipv6Addr::from(segments).to_bits()),
    };
    let length = match prefix.parse::<u8>() {
        Ok(length) if length <= max => length,
        _ => {
            return Err(format!(
                "prefix /{prefix} is too long for {family} (max {max})"
            ));
        }
    };
    let host_mask = match length {
        0 => u128::MAX >> (128 - max),
        _ => (1u128 << (max - length)) - 1,
    };
    if bits & host_mask != 0 {
        let network = bits & !host_mask;
        let network = match ip {
            Ip::V4(_) => Ipv4Addr::from_bits(network as u32).to_string(),
            Ip::V6(_) => Ipv6Addr::from_bits(network).to_string(),
        };
        return Err(format!(
            "host bits are set in {s}; did you mean \"{network}/{length}\"?"
        ));
    }
    Ok((ip, length))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip_err(s: &str) -> String {
        parse_ip(s).expect_err(s)
    }

    fn addr_err(s: &str) -> String {
        parse_socket_addr(s).expect_err(s)
    }

    #[test]
    fn ip_values() {
        assert_eq!(parse_ip("10.0.0.1"), Ok(Ip::V4([10, 0, 0, 1])));
        assert_eq!(parse_ip("::1"), Ok(Ip::V6([0, 0, 0, 0, 0, 0, 0, 1])));
        assert_eq!(
            parse_ip("::ffff:10.0.0.1"),
            Ok(Ip::V6([0, 0, 0, 0, 0, 0xffff, 0x0a00, 0x0001]))
        );
        assert_eq!(
            parse_ip("2001:db8::8a2e:370:7334"),
            Ok(Ip::V6([0x2001, 0xdb8, 0, 0, 0, 0x8a2e, 0x370, 0x7334]))
        );
    }

    #[test]
    fn ip_errors() {
        assert_eq!(
            ip_err("fe80::1%eth0"),
            "zone suffixes like %eth0 can't be stored in Ipv6Addr; remove it"
        );
        assert_eq!(ip_err("hello"), "`hello` is not an IPv4 or IPv6 address");
        assert_eq!(
            ip_err("256.0.0.1"),
            "`256.0.0.1` is not an IPv4 or IPv6 address"
        );
        assert_eq!(ip_err(""), "`` is not an IPv4 or IPv6 address");
    }

    #[test]
    fn ipv4_leading_zeros() {
        assert_eq!(
            ip_err("010.0.0.1"),
            "IPv4 parts can't have leading zeros (is \"010\" decimal or octal?); write 10.0.0.1"
        );
        assert_eq!(
            ip_err("192.168.001.010"),
            "IPv4 parts can't have leading zeros (is \"001\" decimal or octal?); write 192.168.1.10"
        );
    }

    #[test]
    fn ipv4_octal_without_suggestion() {
        assert_eq!(
            ip_err("0300.0.0.1"),
            "IPv4 parts can't have leading zeros (is \"0300\" decimal or octal?)"
        );
    }

    /// Random strings from address-like characters, and random well-formed addresses,
    /// must parse exactly as std parses them.
    #[test]
    fn net_oracle() {
        use std::net::{IpAddr, SocketAddr};
        let mut state = 7u64;
        let mut next = || {
            state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^ (z >> 31)
        };
        let alphabet = b"0123456789abcdef:.[]";
        for i in 0..200_000 {
            let text = match i % 4 {
                0 => std::net::Ipv4Addr::from_bits(next() as u32).to_string(),
                1 => std::net::Ipv6Addr::from_bits(
                    u128::from(next()) << 64 | u128::from(next() >> (next() % 64)),
                )
                .to_string(),
                2 => format!(
                    "{}:{}",
                    std::net::Ipv4Addr::from_bits(next() as u32),
                    next() % 70_000
                ),
                _ => (0..next() % 16)
                    .map(|_| alphabet[(next() % alphabet.len() as u64) as usize] as char)
                    .collect(),
            };
            let ours = parse_ip(&text).ok().map(|ip| match ip {
                Ip::V4(octets) => IpAddr::from(octets),
                Ip::V6(segments) => IpAddr::from(segments),
            });
            assert_eq!(ours, text.parse::<IpAddr>().ok(), "ip {text:?}");
            let ours = parse_socket_addr(&text).ok().map(|(ip, port)| match ip {
                Ip::V4(octets) => SocketAddr::from((octets, port)),
                Ip::V6(segments) => SocketAddr::from((segments, port)),
            });
            assert_eq!(ours, text.parse::<SocketAddr>().ok(), "addr {text:?}");
        }
    }

    #[test]
    fn addr_values() {
        assert_eq!(
            parse_socket_addr("0.0.0.0:8080"),
            Ok((Ip::V4([0, 0, 0, 0]), 8080))
        );
        assert_eq!(
            parse_socket_addr("[::1]:443"),
            Ok((Ip::V6([0, 0, 0, 0, 0, 0, 0, 1]), 443))
        );
        assert_eq!(
            parse_socket_addr("10.0.0.1:65535"),
            Ok((Ip::V4([10, 0, 0, 1]), 65535))
        );
        assert_eq!(
            parse_socket_addr("10.0.0.1:0"),
            Ok((Ip::V4([10, 0, 0, 1]), 0))
        );
    }

    #[test]
    fn addr_errors() {
        assert_eq!(
            addr_err("localhost:80"),
            "`localhost` is a hostname; addr! needs an IP address (looking up a name isn't a literal)"
        );
        assert_eq!(
            addr_err("example.com"),
            "`example.com` is a hostname; addr! needs an IP address (looking up a name isn't a literal)"
        );
        assert_eq!(
            addr_err("::1:443"),
            "IPv6 socket addresses need brackets, like \"[::1]:443\""
        );
        assert_eq!(
            addr_err("10.0.0.1"),
            "missing port; write e.g. \"10.0.0.1:8080\""
        );
        assert_eq!(
            addr_err("10.0.0.1:"),
            "missing port; write e.g. \"10.0.0.1:8080\""
        );
        assert_eq!(addr_err("[::1]"), "missing port; write e.g. \"[::1]:8080\"");
        assert_eq!(
            addr_err("10.0.0.1:65536"),
            "port 65536 is out of range (0-65535)"
        );
        assert_eq!(addr_err("10.0.0.1:http"), "`http` is not a port number");
        assert_eq!(
            addr_err("[10.0.0.1]:80"),
            "brackets are only for IPv6 addresses; write \"10.0.0.1:80\""
        );
    }
}

#[cfg(test)]
mod cidr_tests {
    use super::*;

    fn err(s: &str) -> String {
        parse_cidr(s).expect_err(s)
    }

    #[test]
    fn cidr_values() {
        assert_eq!(parse_cidr("10.0.0.0/8"), Ok((Ip::V4([10, 0, 0, 0]), 8)));
        assert_eq!(
            parse_cidr("2001:db8::/32"),
            Ok((Ip::V6([0x2001, 0xdb8, 0, 0, 0, 0, 0, 0]), 32))
        );
    }

    #[test]
    fn cidr_errors() {
        assert_eq!(
            err("10.1.2.3/8"),
            "host bits are set in 10.1.2.3/8; did you mean \"10.0.0.0/8\"?"
        );
        assert_eq!(
            err("2001:db8::1/32"),
            "host bits are set in 2001:db8::1/32; did you mean \"2001:db8::/32\"?"
        );
        assert_eq!(
            err("10.0.0.0/33"),
            "prefix /33 is too long for IPv4 (max 32)"
        );
        assert_eq!(err("::/129"), "prefix /129 is too long for IPv6 (max 128)");
        assert_eq!(
            err("10.0.0.0/999999999999"),
            "prefix /999999999999 is too long for IPv4 (max 32)"
        );
        assert_eq!(
            err("10.0.0.0"),
            "missing /prefix; write e.g. \"10.0.0.0/8\""
        );
        assert_eq!(
            err("10.0.0.0/"),
            "`/` is not a prefix length; write e.g. \"10.0.0.0/8\""
        );
        assert_eq!(
            err("10.0.0.0/x"),
            "`/x` is not a prefix length; write e.g. \"10.0.0.0/8\""
        );
        assert_eq!(err("10.0.0/8"), "`10.0.0` is not an IPv4 or IPv6 address");
    }

    #[test]
    fn cidr_corners() {
        assert_eq!(parse_cidr("0.0.0.0/0"), Ok((Ip::V4([0; 4]), 0)));
        assert_eq!(parse_cidr("10.0.0.1/32"), Ok((Ip::V4([10, 0, 0, 1]), 32)));
        assert_eq!(parse_cidr("::/0"), Ok((Ip::V6([0; 8]), 0)));
        assert_eq!(
            parse_cidr("::1/128"),
            Ok((Ip::V6([0, 0, 0, 0, 0, 0, 0, 1]), 128))
        );
        assert_eq!(
            err("1.0.0.0/0"),
            "host bits are set in 1.0.0.0/0; did you mean \"0.0.0.0/0\"?"
        );
    }
}
