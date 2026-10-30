//! Procedural macros for `constlit`. Use them through the `constlit` crate.

mod duration;
mod input;
mod net;
mod size;

use quote::quote;

/// A duration literal as a `core::time::Duration` constant.
///
/// See the `constlit` crate documentation for the syntax.
#[proc_macro]
pub fn dur(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    input::expand(input.into(), "\"30s\"", |s| {
        let (secs, nanos) = duration::parse_duration(s)?;
        Ok(quote!(::core::time::Duration::new(#secs, #nanos)))
    })
    .into()
}

/// A byte-size literal as a `constlit::ByteSize` constant.
#[proc_macro]
pub fn size(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    input::expand(input.into(), "\"64 KiB\"", |s| {
        let bytes = size::parse_size(s)?;
        Ok(quote!(::constlit::ByteSize::from_bytes(#bytes)))
    })
    .into()
}

/// An IP address literal as a `core::net::Ipv4Addr` or `Ipv6Addr` constant.
#[proc_macro]
pub fn ip(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    input::expand(input.into(), "\"10.0.0.1\"", |s| {
        Ok(net::parse_ip(s)?.to_tokens())
    })
    .into()
}

/// A socket address literal as a `core::net::SocketAddrV4` or `SocketAddrV6` constant.
#[proc_macro]
pub fn addr(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    input::expand(input.into(), "\"0.0.0.0:8080\"", |s| {
        let (ip, port) = net::parse_socket_addr(s)?;
        let ip_tokens = ip.to_tokens();
        Ok(match ip {
            net::Ip::V4(_) => quote!(::core::net::SocketAddrV4::new(#ip_tokens, #port)),
            net::Ip::V6(_) => quote!(::core::net::SocketAddrV6::new(#ip_tokens, #port, 0, 0)),
        })
    })
    .into()
}

/// A network literal as a `constlit::Ipv4Cidr` or `Ipv6Cidr` constant.
#[proc_macro]
pub fn cidr(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    input::expand(input.into(), "\"10.0.0.0/8\"", |s| {
        let (ip, prefix) = net::parse_cidr(s)?;
        let ip_tokens = ip.to_tokens();
        Ok(match ip {
            net::Ip::V4(_) => quote!(::constlit::Ipv4Cidr::__new_unchecked(#ip_tokens, #prefix)),
            net::Ip::V6(_) => quote!(::constlit::Ipv6Cidr::__new_unchecked(#ip_tokens, #prefix)),
        })
    })
    .into()
}
