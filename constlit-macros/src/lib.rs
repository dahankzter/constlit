//! Procedural macros for `constlit`. Use them through the `constlit` crate.
#![forbid(unsafe_code)]

mod cap;
mod cron;
#[cfg(test)]
#[path = "../../constlit/tests/support/cron_gen.rs"]
mod cron_gen;
mod datetime;
mod docker_port;
mod duration;
mod errno;
mod input;
mod k8s_names;
mod linux_misc;
mod mac;
mod media_type;
mod mode;
mod net;
mod oci_digest;
mod oci_image;
mod oci_platform;
mod quantity;
mod selector;
mod signal;
mod size;
mod suggest;
mod systemd_calendar;
mod systemd_timespan;
mod systemd_unit;
mod ts;
mod version;
mod version_req;
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

/// A MAC address literal as a `constlit::MacAddr` constant.
#[proc_macro]
pub fn mac(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    input::expand(input.into(), "\"00:1a:2b:3c:4d:5e\"", |s| {
        let bytes = mac::parse_mac(s)?;
        Ok(quote!(::constlit::MacAddr::new([#(#bytes),*])))
    })
    .into()
}

/// A file permission mode literal as a `constlit::unix::Mode` constant.
/// Use it as `constlit::unix::mode!`.
#[proc_macro]
pub fn unix_mode(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    input::expand(input.into(), "\"0755\"", |s| {
        let bits = mode::parse_mode(s)?;
        Ok(quote!(::constlit::unix::Mode::__new_unchecked(#bits)))
    })
    .into()
}

/// A signal name literal as a `constlit::unix::Signal` constant, numbered for the
/// build target. Use it as `constlit::unix::signal!`.
#[proc_macro]
pub fn unix_signal(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    input::expand_spanned(input.into(), "\"SIGTERM\"", |s, span| {
        Ok(signal::expand_signal(signal::parse_signal(s)?, s, span))
    })
    .into()
}

/// A Linux capability name literal as a `constlit::linux::Capability` constant.
/// Use it as `constlit::linux::cap!`.
#[proc_macro]
pub fn linux_cap(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    input::expand(input.into(), "\"CAP_NET_ADMIN\"", |s| {
        let number = cap::parse_cap(s)?;
        Ok(quote!(::constlit::linux::Capability::__new_unchecked(#number)))
    })
    .into()
}

/// A list of Linux capability names as a `constlit::linux::CapSet` constant.
/// Use it as `constlit::linux::caps!`.
#[proc_macro]
pub fn linux_caps(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    input::expand(input.into(), "\"CAP_NET_ADMIN, CAP_SYS_TIME\"", |s| {
        let bits = cap::parse_caps(s)?;
        Ok(quote!(::constlit::linux::CapSet::__from_bits_unchecked(#bits)))
    })
    .into()
}

/// A Kubernetes quantity literal as a `constlit::k8s::Quantity` constant. Use it as `constlit::k8s::quantity!`.
#[proc_macro]
pub fn k8s_quantity(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    input::expand(input.into(), "\"1.5Gi\"", |s| {
        let millis = quantity::parse_quantity(s)?.millis;
        Ok(quote!(::constlit::k8s::Quantity::__new_unchecked(#millis, #s)))
    })
    .into()
}

/// A Kubernetes CPU amount as a `constlit::k8s::Quantity` constant. Use it as `constlit::k8s::cpu!`.
#[proc_macro]
pub fn k8s_cpu(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    input::expand(input.into(), "\"500m\"", |s| {
        let millis = quantity::parse_cpu(s)?;
        Ok(quote!(::constlit::k8s::Quantity::__new_unchecked(#millis, #s)))
    })
    .into()
}

/// A Kubernetes memory or storage amount as a `constlit::k8s::Quantity` constant. Use it as `constlit::k8s::mem!`.
#[proc_macro]
pub fn k8s_mem(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    input::expand(input.into(), "\"512Mi\"", |s| {
        let millis = quantity::parse_mem(s)?;
        Ok(quote!(::constlit::k8s::Quantity::__new_unchecked(#millis, #s)))
    })
    .into()
}

/// An RFC 1123 label (Kubernetes namespace and most object names), checked, as a `&'static str`. Use it as `constlit::k8s::dns_label!`.
#[proc_macro]
pub fn k8s_dns_label(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    input::expand(input.into(), "\"my-app\"", |s| {
        k8s_names::check_dns_label(s)?;
        Ok(quote!(#s))
    })
    .into()
}

/// An RFC 1035 label (Kubernetes Service names), checked, as a `&'static str`. Use it as `constlit::k8s::dns1035_label!`.
#[proc_macro]
pub fn k8s_dns1035_label(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    input::expand(input.into(), "\"web\"", |s| {
        k8s_names::check_dns1035_label(s)?;
        Ok(quote!(#s))
    })
    .into()
}

/// An RFC 1123 subdomain (Kubernetes ConfigMap, Secret and other names), checked, as a `&'static str`. Use it as `constlit::k8s::dns_subdomain!`.
#[proc_macro]
pub fn k8s_dns_subdomain(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    input::expand(input.into(), "\"app.config.v1\"", |s| {
        k8s_names::check_dns_subdomain(s)?;
        Ok(quote!(#s))
    })
    .into()
}

/// A Kubernetes label key, checked, as a `&'static str`. Use it as `constlit::k8s::label_key!`.
#[proc_macro]
pub fn k8s_label_key(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    input::expand(input.into(), "\"app.kubernetes.io/name\"", |s| {
        k8s_names::check_label_key(s)?;
        Ok(quote!(#s))
    })
    .into()
}

/// A Kubernetes label value, checked, as a `&'static str`. Use it as `constlit::k8s::label_value!`.
#[proc_macro]
pub fn k8s_label_value(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    input::expand(input.into(), "\"web\"", |s| {
        k8s_names::check_label_value(s)?;
        Ok(quote!(#s))
    })
    .into()
}

/// A Kubernetes label selector, checked, as a `&'static str`. Use it as
/// `constlit::k8s::selector!`.
#[proc_macro]
pub fn k8s_selector(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    input::expand(input.into(), "\"app=web,env in (prod,staging)\"", |s| {
        selector::check_selector(s)?;
        Ok(quote!(#s))
    })
    .into()
}

/// An OCI content digest literal as a `constlit::oci::Digest` constant. Use it as
/// `constlit::oci::digest!`.
#[proc_macro]
pub fn oci_digest(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    input::expand(input.into(), "\"sha256:...\"", |s| {
        let (algorithm, bytes) = oci_digest::parse_digest(s)?;
        Ok(oci_digest::digest_tokens(algorithm, &bytes))
    })
    .into()
}

/// A container image reference literal as a `constlit::oci::ImageRef` constant.
/// Use it as `constlit::oci::image!`.
#[proc_macro]
pub fn oci_image(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    input::expand(input.into(), "\"ghcr.io/org/app:1.2\"", |s| {
        let image = oci_image::parse_image(s)?;
        let (registry, repository, canonical) =
            (&image.registry, &image.repository, &image.canonical);
        let tag = match &image.tag {
            Some(tag) => quote!(::core::option::Option::Some(#tag)),
            None => quote!(::core::option::Option::None),
        };
        let digest = match &image.digest {
            Some((algorithm, bytes, _)) => {
                let digest = oci_digest::digest_tokens(*algorithm, bytes);
                quote!(::core::option::Option::Some(#digest))
            }
            None => quote!(::core::option::Option::None),
        };
        Ok(quote!(::constlit::oci::ImageRef::__new_unchecked(
            #s, #registry, #repository, #tag, #digest, #canonical
        )))
    })
    .into()
}

/// An OCI platform literal as a `constlit::oci::Platform` constant. Use it as
/// `constlit::oci::platform!`.
#[proc_macro]
pub fn oci_platform(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    input::expand(input.into(), "\"linux/amd64\"", |s| {
        let (os, arch, variant) = oci_platform::parse_platform(s)?;
        let variant = match variant {
            Some(v) => quote!(::core::option::Option::Some(#v)),
            None => quote!(::core::option::Option::None),
        };
        Ok(quote!(::constlit::oci::Platform::__new_unchecked(#os, #arch, #variant)))
    })
    .into()
}

/// A Docker port mapping literal as a `constlit::docker::PortMapping` constant. Use
/// it as `constlit::docker::port!`.
#[proc_macro]
pub fn docker_port(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    input::expand(input.into(), "\"8080:80\"", |s| {
        let port = docker_port::parse_port(s)?;
        let none = quote!(::core::option::Option::None);
        let ip = match port.host_ip {
            Some(std::net::IpAddr::V4(ip)) => {
                let [a, b, c, d] = ip.octets();
                quote!(::core::option::Option::Some(::core::net::IpAddr::V4(
                    ::core::net::Ipv4Addr::new(#a, #b, #c, #d)
                )))
            }
            Some(std::net::IpAddr::V6(ip)) => {
                let [a, b, c, d, e, f, g, h] = ip.segments();
                quote!(::core::option::Option::Some(::core::net::IpAddr::V6(
                    ::core::net::Ipv6Addr::new(#a, #b, #c, #d, #e, #f, #g, #h)
                )))
            }
            None => none.clone(),
        };
        let range = |(start, end): (u16, u16)| {
            quote!(::constlit::docker::PortRange::__new_unchecked(#start, #end))
        };
        let host = match port.host {
            Some(host) => {
                let host = range(host);
                quote!(::core::option::Option::Some(#host))
            }
            None => none,
        };
        let container = range(port.container);
        let protocol = match port.protocol {
            "udp" => quote!(::constlit::docker::Protocol::Udp),
            "sctp" => quote!(::constlit::docker::Protocol::Sctp),
            _ => quote!(::constlit::docker::Protocol::Tcp),
        };
        Ok(quote!(::constlit::docker::PortMapping::__new_unchecked(
            #ip, #host, #container, #protocol
        )))
    })
    .into()
}

/// An errno name literal as a `constlit::linux::Errno` constant, numbered for the
/// build target. Use it as `constlit::linux::errno!`.
#[proc_macro]
pub fn linux_errno(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    input::expand_spanned(input.into(), "\"ENOENT\"", |s, span| {
        Ok(errno::expand_errno(errno::parse_errno(s)?, span))
    })
    .into()
}

/// A Linux network interface name literal, checked, as a `&'static str`. Use it as
/// `constlit::linux::ifname!`.
#[proc_macro]
pub fn linux_ifname(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    input::expand(input.into(), "\"eth0\"", |s| {
        linux_misc::check_ifname(s)?;
        Ok(quote!(#s))
    })
    .into()
}

/// A Linux device number literal as a `constlit::linux::DevNum` constant. Use it as
/// `constlit::linux::dev!`.
#[proc_macro]
pub fn linux_dev(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    input::expand(input.into(), "\"8:0\"", |s| {
        let (major, minor) = linux_misc::parse_dev(s)?;
        Ok(quote!(::constlit::linux::DevNum::__new_unchecked(#major, #minor)))
    })
    .into()
}

/// A systemd time span literal as a `core::time::Duration` constant. Use it as
/// `constlit::systemd::timespan!`.
#[proc_macro]
pub fn systemd_timespan(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    input::expand(input.into(), "\"5min 30s\"", |s| {
        let (secs, nanos) = systemd_timespan::parse_timespan(s)?;
        Ok(quote!(::core::time::Duration::new(#secs, #nanos)))
    })
    .into()
}

/// A systemd unit name literal, checked, as a `&'static str`. Use it as
/// `constlit::systemd::unit!`.
#[proc_macro]
pub fn systemd_unit(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    input::expand(input.into(), "\"nginx.service\"", |s| {
        systemd_unit::check_unit(s)?;
        Ok(quote!(#s))
    })
    .into()
}

/// A systemd calendar expression literal as a `constlit::systemd::CalendarSpec`
/// constant. Use it as `constlit::systemd::calendar!`.
#[proc_macro]
pub fn systemd_calendar(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    input::expand(input.into(), "\"Mon..Fri 09:00\"", |s| {
        let c = systemd_calendar::parse_calendar(s)?;
        let [y0, y1, y2, y3] = c.years;
        let (weekdays, months, days, from_end) = (c.weekdays, c.months, c.days, c.from_end);
        let (hours, minutes, seconds, utc, normalized) =
            (c.hours, c.minutes, c.seconds, c.utc, c.normalized);
        Ok(quote!(::constlit::systemd::CalendarSpec::__new_unchecked(
            #weekdays, [#y0, #y1, #y2, #y3], #months, #days, #from_end,
            #hours, #minutes, #seconds, #utc, #normalized
        )))
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

/// A Cargo version requirement literal as a `constlit::VersionReq` constant.
#[proc_macro]
pub fn version_req(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    input::expand(input.into(), "\">=1.2, <2\"", |s| {
        let comparators = version_req::parse_version_req(s)?.into_iter().map(|c| {
            let op = match c.op {
                "=" => quote!(Exact),
                ">" => quote!(Greater),
                ">=" => quote!(GreaterEq),
                "<" => quote!(Less),
                "<=" => quote!(LessEq),
                "~" => quote!(Tilde),
                "*" => quote!(Wildcard),
                _ => quote!(Caret),
            };
            let option = |n: Option<u64>| match n {
                Some(n) => quote!(::core::option::Option::Some(#n)),
                None => quote!(::core::option::Option::None),
            };
            let (major, minor, patch, pre) = (c.major, option(c.minor), option(c.patch), c.pre);
            quote!(::constlit::Comparator::__new_unchecked(
                ::constlit::Op::#op, #major, #minor, #patch, #pre
            ))
        });
        // A named const item, so the comparator slice is 'static in any position.
        Ok(quote!({
            const __CONSTLIT_VERSION_REQ: ::constlit::VersionReq =
                ::constlit::VersionReq::__new_unchecked(&[#(#comparators),*], #s);
            __CONSTLIT_VERSION_REQ
        }))
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
