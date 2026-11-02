//! Compile-time checked literals that become typed constants.
#![no_std]

#[cfg(feature = "mime")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

mod calendar;
mod cidr;
mod cron;
#[cfg(any(feature = "jiff", feature = "chrono"))]
mod cron_time;
mod date;
mod media_type;
mod size;
mod time_of_day;
mod timestamp;
mod version;
mod zoned;

pub use cidr::{Cidr, CidrError, Ipv4Cidr, Ipv6Cidr};
pub use cron::CronSpec;
#[cfg(any(feature = "jiff", feature = "chrono"))]
pub use cron_time::CronTime;
pub use date::{Date, Weekday};
pub use media_type::MediaType;
pub use size::ByteSize;
pub use time_of_day::TimeOfDay;
pub use timestamp::Timestamp;
pub use version::Version;
pub use zoned::ZonedTimestamp;

/// A duration literal, checked at compile time, as a [`core::time::Duration`].
///
/// Units: `ns`, `us` (or `µs`), `ms`, `s`, `m`, `h`, `d` (exactly 24 hours).
/// Components go from largest to smallest, optionally separated by single spaces.
/// Decimals are allowed when the result is a whole number of nanoseconds.
///
/// ```
/// use core::time::Duration;
/// const TIMEOUT: Duration = constlit::dur!("1h30m");
/// assert_eq!(TIMEOUT, Duration::from_secs(5400));
/// assert_eq!(constlit::dur!("1.5s"), Duration::from_millis(1500));
/// ```
///
/// A bare number, an unknown unit, or units out of order is a compile error:
///
/// ```compile_fail
/// let _ = constlit::dur!("30");
/// ```
pub use constlit_macros::dur;

/// A byte-size literal, checked at compile time, as a [`ByteSize`].
///
/// Units are strict and case-sensitive:
/// - `KiB`, `MiB`, `GiB`, `TiB`, `PiB` (and Kubernetes' `Ki` ... `Pi`) are powers of 1024;
/// - `KB`, `MB`, `GB`, `TB`, `PB` are powers of 1000;
/// - `B`, or no unit, is bytes.
///
/// ```
/// use constlit::{ByteSize, size};
/// const LIMIT: ByteSize = size!("64 KiB");
/// assert_eq!(LIMIT.bytes(), 65536);
/// assert_eq!(size!("1.5 KB").bytes(), 1500);
/// ```
///
/// Any other spelling, like `kb`, is a compile error asking which one you meant:
///
/// ```compile_fail
/// let _ = constlit::size!("64 kb");
/// ```
pub use constlit_macros::size;

/// An IP address literal, checked at compile time.
///
/// Dotted IPv4 becomes a [`core::net::Ipv4Addr`]; any standard IPv6 form becomes
/// a [`core::net::Ipv6Addr`]. Use `.into()` for an `IpAddr`.
///
/// ```
/// use core::net::{Ipv4Addr, Ipv6Addr};
/// const DNS: Ipv4Addr = constlit::ip!("10.0.0.53");
/// const LOOPBACK: Ipv6Addr = constlit::ip!("::1");
/// assert_eq!(LOOPBACK, Ipv6Addr::LOCALHOST);
/// ```
///
/// Leading zeros (`"010.0.0.1"`: decimal or octal?) and zone suffixes are compile errors:
///
/// ```compile_fail
/// let _ = constlit::ip!("010.0.0.1");
/// ```
pub use constlit_macros::ip;

/// A socket address literal, checked at compile time.
///
/// `"ipv4:port"` becomes a [`core::net::SocketAddrV4`]; `"[ipv6]:port"` becomes a
/// [`core::net::SocketAddrV6`]. Hostnames are compile errors: they need a DNS lookup.
///
/// ```
/// use core::net::SocketAddrV4;
/// const LISTEN: SocketAddrV4 = constlit::addr!("0.0.0.0:8080");
/// assert_eq!(LISTEN.port(), 8080);
/// ```
///
/// ```compile_fail
/// let _ = constlit::addr!("localhost:80");
/// ```
pub use constlit_macros::addr;

/// A network literal, checked at compile time, as an [`Ipv4Cidr`] or [`Ipv6Cidr`].
///
/// The address must be the network address: host bits set after the prefix are
/// a compile error that suggests the network you probably meant.
///
/// ```
/// use constlit::{Ipv4Cidr, cidr};
/// const PRIVATE: Ipv4Cidr = cidr!("10.0.0.0/8");
/// assert!(PRIVATE.contains(constlit::ip!("10.1.2.3")));
/// assert_eq!(PRIVATE.broadcast(), constlit::ip!("10.255.255.255"));
/// ```
///
/// ```compile_fail
/// let _ = constlit::cidr!("10.1.2.3/8"); // did you mean "10.0.0.0/8"?
/// ```
pub use constlit_macros::cidr;

/// A version literal, checked at compile time against
/// [SemVer 2.0.0](https://semver.org), as a [`Version`].
///
/// The grammar is SemVer's to the letter: `MAJOR.MINOR.PATCH`, then an optional
/// `-pre.release` and `+build.metadata`. Comparison uses SemVer precedence.
///
/// ```
/// use constlit::{Version, version};
/// const MIN: Version = version!("1.4.0-rc.2");
/// assert!(MIN < version!("1.4.0"));
/// assert_eq!(MIN.pre(), "rc.2");
/// ```
///
/// A leading `v`, as in git tags, is a compile error: SemVer says `v1.2.3` is not a
/// semantic version, the `v` belongs to the tag name
/// ([FAQ](https://semver.org/#is-v123-a-semantic-version)). Write `"1.2.3"`.
///
/// ```compile_fail
/// let _ = constlit::version!("v1.2.3");
/// ```
pub use constlit_macros::version;

/// A media type literal (MIME type), checked at compile time, as a [`MediaType`].
///
/// The grammar is `type/subtype`, then optional `; name=value` parameters, where a
/// value is a token or a quoted string. Names are case-insensitive and stored lowercase.
///
/// ```
/// use constlit::{MediaType, mime};
/// const HTML: MediaType = mime!("text/html; charset=UTF-8");
/// assert_eq!(HTML.essence(), "text/html");
/// assert_eq!(HTML.param("charset"), Some("utf-8"));
/// ```
///
/// Wildcards like `"text/*"` are media ranges for `Accept` headers, not media types,
/// so they are a compile error:
///
/// ```compile_fail
/// let _ = constlit::mime!("text/*");
/// ```
pub use constlit_macros::mime;

/// A cron expression, checked at compile time, as a [`CronSpec`].
///
/// Five fields, `minute hour day-of-month month day-of-week`, separated by spaces
/// or tabs. Each is a `,`-list of `*`, `*/n`, `a`, `a-b`, `a-b/n` or `a/n`
/// (`a` to the field's end in steps of `n`). Months and weekdays also take names
/// (`JAN`, `mon`); day of week is 0-7, where both 0 and 7 are Sunday. The shortcuts
/// `@yearly`, `@annually`, `@monthly`, `@weekly`, `@daily`, `@midnight` and
/// `@hourly` work too.
///
/// ```
/// use constlit::{CronSpec, cron};
/// const WORKDAYS: CronSpec = cron!("0 9-17 * * MON-FRI");
/// // 9:00 on Friday 30 October 2026
/// assert!(WORKDAYS.matches(0, 9, 30, 10, 5));
/// assert!(!WORKDAYS.matches(0, 9, 31, 10, 6));
/// ```
///
/// With the `jiff` or `chrono` feature, `CronSpec::next_after` finds the next run
/// time after a date-time, honoring daylight-saving changes in zoned types.
///
/// A seconds field, Quartz syntax (`?`, `L`, `W`, `#`), `@reboot` and values out of
/// range are compile errors:
///
/// ```compile_fail
/// let _ = constlit::cron!("0 */5 * * * *");
/// ```
pub use constlit_macros::cron;

/// A calendar date literal, checked at compile time, as a [`Date`].
///
/// The form is ISO 8601 `YYYY-MM-DD`, with years 0001-9999 in the proleptic
/// Gregorian calendar.
///
/// ```
/// use constlit::{Date, Weekday, date};
/// const LAUNCH: Date = date!("2026-10-30");
/// assert_eq!(LAUNCH.weekday(), Weekday::Friday);
/// ```
///
/// Impossible dates are compile errors that name the month's length, and so are
/// other orders such as `"30.10.2026"`:
///
/// ```compile_fail
/// let _ = constlit::date!("2026-02-30");
/// ```
pub use constlit_macros::date;

/// A time-of-day literal, checked at compile time, as a [`TimeOfDay`].
///
/// The form is `HH:MM`, `HH:MM:SS` or `HH:MM:SS.F` with 1-9 fraction digits, on
/// the 24-hour clock.
///
/// ```
/// use constlit::{TimeOfDay, time};
/// const STANDUP: TimeOfDay = time!("09:15");
/// assert_eq!(STANDUP.to_string(), "09:15:00");
/// ```
///
/// `24:00`, leap seconds (`:60`), AM/PM and zones are compile errors:
///
/// ```compile_fail
/// let _ = constlit::time!("24:00");
/// ```
pub use constlit_macros::time;

/// A timestamp literal, checked at compile time, as a [`Timestamp`], or a
/// [`ZonedTimestamp`] when it names a time zone.
///
/// The form is `YYYY-MM-DD`, `T` or one space, `HH:MM[:SS[.F]]`, then `Z` for
/// UTC or an offset `±HH:MM` (also `±HHMM` and `±HH:MM:SS`):
///
/// ```
/// use constlit::{Timestamp, ts};
/// const LAUNCH: Timestamp = ts!("2026-10-30T14:00+01:00");
/// assert_eq!(LAUNCH.unix_seconds(), 1_793_365_200);
/// assert_eq!(LAUNCH, ts!("2026-10-30 13:00Z"));
/// ```
///
/// A timestamp without a zone is ambiguous, so it is a compile error:
///
/// ```compile_fail
/// let _ = constlit::ts!("2026-10-30 14:00");
/// ```
///
/// # Named time zones
///
/// With the `tz` feature (on by default), an IANA zone name may follow the time,
/// or an offset, after one space. The zone is resolved while your crate compiles,
/// and the result is a [`ZonedTimestamp`]:
///
/// ```
/// # #[cfg(feature = "tz")] {
/// use constlit::{ZonedTimestamp, ts};
/// const MEETING: ZonedTimestamp = ts!("2026-10-30 14:00 Europe/Stockholm");
/// assert_eq!(MEETING.offset_seconds(), 3600);
/// # }
/// ```
///
/// A local time skipped when the clocks go forward is a compile error. So is one
/// that happens twice when they go back, unless an offset picks one; the message
/// gives both spellings:
///
/// ```compile_fail
/// let _ = constlit::ts!("2026-10-25 02:30 Europe/Stockholm");
/// ```
///
/// An explicit offset that the zone doesn't have at that time is an error too.
///
/// **The zone rules are the ones known when `constlit` was built.** For a future
/// instant, a later change to a zone's law takes effect only after a rebuild with
/// a newer `constlit`.
#[doc = concat!(
    "The bundled IANA time-zone database is version ",
    constlit_macros::__tzdb_version!(),
    "."
)]
pub use constlit_macros::ts;
