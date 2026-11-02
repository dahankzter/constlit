//! Procedural macros for `constlit`. Use them through the `constlit` crate.

mod cron;
#[cfg(test)]
#[path = "../../constlit/tests/support/cron_gen.rs"]
mod cron_gen;
mod datetime;
mod duration;
mod input;
mod media_type;
mod net;
mod size;
mod ts;
mod version;
#[cfg(feature = "tz")]
mod zone;

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

/// A SemVer 2.0.0 version literal as a `constlit::Version` constant.
#[proc_macro]
pub fn version(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    input::expand(input.into(), "\"1.2.3\"", |s| {
        let (major, minor, patch, pre, build) = version::parse_version(s)?;
        Ok(quote!(::constlit::Version::__new_unchecked(#major, #minor, #patch, #pre, #build)))
    })
    .into()
}

/// A media type literal as a `constlit::MediaType` constant.
#[proc_macro]
pub fn mime(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    input::expand(input.into(), "\"application/json\"", |s| {
        let (essence, slash, params) = media_type::parse_media_type(s)?;
        let params = params.iter().map(|(name, value)| quote!((#name, #value)));
        Ok(quote!(::constlit::MediaType::__new_unchecked(#essence, #slash, &[#(#params),*])))
    })
    .into()
}

/// A cron expression literal as a `constlit::CronSpec` constant.
#[proc_macro]
pub fn cron(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    input::expand(input.into(), "\"*/5 * * * *\"", |s| {
        let cron::Cron {
            minutes,
            hours,
            days,
            months,
            weekdays,
            dom_star,
            dow_star,
            time_star,
        } = cron::parse_cron(s)?;
        Ok(quote!(::constlit::CronSpec::__new_unchecked(
            #s, #minutes, #hours, #days, #months, #weekdays, #dom_star, #dow_star, #time_star
        )))
    })
    .into()
}

/// A date literal as a `constlit::Date` constant.
#[proc_macro]
pub fn date(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    input::expand(input.into(), "\"2026-10-30\"", |s| {
        let (year, month, day) = datetime::parse_date(s)?;
        Ok(quote!(::constlit::Date::__new_unchecked(#year, #month, #day)))
    })
    .into()
}

/// A time-of-day literal as a `constlit::TimeOfDay` constant.
#[proc_macro]
pub fn time(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    input::expand(input.into(), "\"14:30\"", |s| {
        let (hour, minute, second, nanosecond) = datetime::parse_time(s)?;
        Ok(quote!(::constlit::TimeOfDay::__new_unchecked(
            #hour, #minute, #second, #nanosecond
        )))
    })
    .into()
}

/// A timestamp literal as a `constlit::Timestamp` constant, or a
/// `constlit::ZonedTimestamp` for a named zone.
#[proc_macro]
pub fn ts(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    input::expand(input.into(), "\"2026-10-30T14:00Z\"", |s| {
        Ok(match ts::parse_ts(s)? {
            ts::Ts::Fixed {
                secs,
                nanos,
                offset,
            } => quote!(::constlit::Timestamp::__new_unchecked(#secs, #nanos, #offset)),
            ts::Ts::Zoned {
                secs,
                nanos,
                offset,
                zone,
            } => quote!(::constlit::ZonedTimestamp::__new_unchecked(
                ::constlit::Timestamp::__new_unchecked(#secs, #nanos, #offset),
                #zone
            )),
        })
    })
    .into()
}

/// The bundled IANA time-zone database version, as a string literal, for the docs.
#[doc(hidden)]
#[proc_macro]
pub fn __tzdb_version(_input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    #[cfg(feature = "tz")]
    let version = zone::tzdb_version();
    #[cfg(not(feature = "tz"))]
    let version = "none (the tz feature is off)";
    let literal = proc_macro2::Literal::string(version);
    quote!(#literal).into()
}
