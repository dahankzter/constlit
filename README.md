# constlit

[![crates.io](https://img.shields.io/crates/v/constlit.svg)](https://crates.io/crates/constlit)
[![docs.rs](https://img.shields.io/docsrs/constlit)](https://docs.rs/constlit)
[![license](https://img.shields.io/crates/l/constlit.svg)](#license)

Readable string literals that become typed constants at compile time.

```rust
use constlit::{ByteSize, Timestamp, ZonedTimestamp, dur, ip, size, ts};
use std::net::Ipv4Addr;
use std::time::Duration;

const TIMEOUT: Duration = dur!("1h30m");
const LIMIT: ByteSize = size!("64 KiB");
const DNS: Ipv4Addr = ip!("10.0.0.53");
const DEADLINE: Timestamp = ts!("2026-10-30T14:00Z");
const LAUNCH: ZonedTimestamp = ts!("2026-10-30 14:00 Europe/Stockholm");

assert_eq!(TIMEOUT.as_secs(), 5400);
assert_eq!(LIMIT.bytes(), 65_536);
assert_eq!(LAUNCH.unix_seconds(), DEADLINE.unix_seconds() - 3600);
```

Each macro parses its literal while your crate compiles:

- **Checked.** A malformed literal is a compile error that points at the string, says
  what is wrong, and suggests a valid literal.
- **Free at run time.** A valid literal becomes a constant of a real type. Nothing is
  parsed, allocated or looked up when the program runs, not even time zones.
- **Strict.** A literal that could mean two things is an error, never a guess: `"64 kb"`
  (1000 or 1024?), `"30"` (seconds?), a local time that happens twice when the clocks
  go back.
- **`no_std`.** The runtime crate needs no allocator and has no required dependencies.

## Installation

```sh
cargo add constlit
```

## The macros

| macro | example | value |
|---|---|---|
| `dur!` | `dur!("1h30m")`, `dur!("250ms")` | `core::time::Duration` |
| `size!` | `size!("64 KiB")`, `size!("1.5 GB")` | `constlit::ByteSize` |
| `ip!` | `ip!("10.0.0.1")`, `ip!("::1")` | `Ipv4Addr` or `Ipv6Addr`, by literal |
| `addr!` | `addr!("0.0.0.0:8080")`, `addr!("[::1]:443")` | `SocketAddrV4` or `SocketAddrV6`, by literal |
| `cidr!` | `cidr!("10.0.0.0/8")` | `constlit::Ipv4Cidr` or `Ipv6Cidr`, by literal |
| `version!` | `version!("1.2.3-beta.1+build.5")` | `constlit::Version` |
| `mime!` | `mime!("text/html; charset=utf-8")` | `constlit::MediaType` |
| `cron!` | `cron!("*/5 9-17 * * MON-FRI")` | `constlit::CronSpec` |
| `date!` | `date!("2026-10-30")` | `constlit::Date` |
| `time!` | `time!("14:30:05.250")` | `constlit::TimeOfDay` |
| `ts!` | `ts!("2026-10-30T14:00+01:00")`, `ts!("2026-10-30 14:00 Europe/Stockholm")` | `constlit::Timestamp` or `ZonedTimestamp`, by literal |

Every macro works in `const` and `static` items. The crate's own types keep their
fields private, so every value is valid: there is no February 30 and no CIDR with
host bits set. Their accessors are `const fn`, and they have checked constructors
(`Date::new`, `Ipv4Cidr::new`, `Version::new`, ...) for values computed at run time.

## Errors

The error sits on the literal and names the fix:

```text
error: `kb` is ambiguous: write `KiB` (1024 bytes) or `KB` (1000 bytes)
 --> src/main.rs:2:29
  |
2 |     let _ = constlit::size!("64 kb");
  |                             ^^^^^^^
```

A few more:

| literal | error |
|---|---|
| `dur!("30")` | ``missing unit after 30; write e.g. "30s" or "30ms"`` |
| `cidr!("10.1.2.3/8")` | ``host bits are set in 10.1.2.3/8; did you mean "10.0.0.0/8"?`` |
| `version!("v1.2.3")` | ``remove the leading "v"; write "1.2.3"`` |
| `date!("2026-02-30")` | ``day 30 is out of range: February 2026 has 28 days`` |
| `time!("2:30 PM")` | ``use the 24-hour clock: "14:30"`` |
| `ts!("2026-10-25 02:30 Europe/Stockholm")` | ``2026-10-25 02:30 happens twice in Europe/Stockholm; write "2026-10-25 02:30+02:00 Europe/Stockholm" (the first) or "2026-10-25 02:30+01:00 Europe/Stockholm" (the second)`` |

## The rules

- **`dur!`:** units `ns`, `us` (or `µs`), `ms`, `s`, `m`, `h`, `d` (exactly 24 hours),
  largest first, optionally separated by single spaces: `"1h 30m"`. Decimals are
  allowed when the result is a whole number of nanoseconds (`"1.5h"`). Weeks, months
  and years have no fixed length, so they are errors.
- **`size!`:** `KiB`, `MiB`, `GiB`, `TiB`, `PiB` (and Kubernetes' `Ki` ... `Pi`) are
  powers of 1024; `KB`, `MB`, `GB`, `TB`, `PB` are powers of 1000; `B` or a plain
  number is bytes. Any other spelling, such as `kb` or `KIB`, is an error. The value
  must fit in a `u64`.
- **`ip!`, `addr!`, `cidr!`:** standard textual forms. IPv6 in `addr!` needs brackets.
  Zone suffixes (`%eth0`), hostnames and CIDRs with host bits set are errors. Use
  `.into()` for `IpAddr`, `SocketAddr` or `Cidr`.
- **`version!`:** [SemVer 2.0.0](https://semver.org) exactly, so `"v1.2.3"` and `"1.2"`
  are errors. `Ord`, `Eq` and `Hash` follow SemVer precedence and ignore build metadata.
- **`mime!`:** RFC 6838 type and subtype, RFC 9110 parameters (tokens or quoted
  strings). Names are lowercased, and so is the `charset` value. Wildcards like
  `"text/*"` are media ranges, not media types, so they are errors.
- **`cron!`:** five-field cron with lists, ranges, steps, `JAN`-`DEC` and `SUN`-`SAT`,
  plus `@yearly` ... `@hourly`. A seconds field, Quartz syntax (`?`, `L`, `W`, `#`) and
  `@reboot` are errors.
- **`date!`:** ISO 8601 `YYYY-MM-DD`, years 0001-9999. Day-first and US orders,
  two-digit years and week dates are errors.
- **`time!`:** `HH:MM`, `HH:MM:SS` or `HH:MM:SS.F` (up to 9 fraction digits) on the
  24-hour clock. `24:00`, leap seconds and AM/PM are errors.
- **`ts!`:** a date, `T` or one space, a time, and then `Z`, an offset (`+01:00`,
  `-0530`), or a space and an IANA zone name. A timestamp without a zone is an error.

## Time zones

With the default `tz` feature, `ts!` resolves zone names while your crate compiles,
using the IANA database bundled with [jiff](https://crates.io/crates/jiff), never
the build machine's:

- A local time skipped when the clocks go forward is an error that shows the jump.
- A local time that happens twice when they go back is an error offering both
  spellings, unless an offset picks one: `ts!("2026-10-25 02:30+01:00 Europe/Stockholm")`.
- An offset the zone doesn't have at that time is an error.

**The rules are the ones known when `constlit` was built.** For a future instant, a
later change to a zone's law takes effect only after you rebuild with a newer
`constlit`. The [`ts!` docs](https://docs.rs/constlit/latest/constlit/macro.ts.html)
name the database version.

Without `tz` (`default-features = false`), `ts!` takes `Z` and offsets only, and jiff
isn't built at all.

## Cron schedules

`CronSpec::matches` checks a minute against the schedule, using Vixie cron's rule for
the two day fields: when either starts with `*`, both must match; otherwise either is
enough. With the `jiff` or `chrono` feature, `next_after` finds the next run:

```rust,ignore
use constlit::cron;

let spec = cron!("*/15 9-17 * * MON-FRI");
let now: jiff::Zoned = "2026-10-30T09:07[Europe/Stockholm]".parse()?;
assert_eq!(spec.next_after(&now).unwrap().to_string(), "2026-10-30T09:15:00+01:00[Europe/Stockholm]");
```

On zoned date-times it follows Vixie cron through daylight-saving changes:

- A schedule with a fixed minute and hour (`30 2 * * *`) runs once when its time is
  skipped (right after the gap) and once when it is repeated (the first time).
- A schedule whose minute or hour field starts with `*` (`*/5 * * * *`, `@hourly`)
  runs on real time: skipped minutes don't run, and a repeated hour runs twice.
  `cron!("30 * * * *").catch_up_gaps()` makes skipped times run after the gap instead.

## Features

| feature | default | adds |
|---|---|---|
| `tz` | yes | named time zones in `ts!` (pulls jiff into the proc macro only) |
| `std` | | `Timestamp::to_system_time` |
| `jiff` | | `CronSpec::next_after` for jiff date-times; `to_jiff` on `Date`, `TimeOfDay` and `Timestamp`; `ZonedTimestamp::to_jiff_zoned` |
| `chrono` | | `CronSpec::next_after` for chrono date-times; `to_chrono` on `Date`, `TimeOfDay` and `Timestamp` |
| `ipnet` | | `From` the CIDR types into `ipnet::Ipv4Net` and `Ipv6Net` |
| `semver` | | `Version::to_semver` |
| `mime` | | `MediaType::to_mime` (needs `std`) |

## Platforms

`no_std` with no allocation; it builds for bare-metal targets such as
`thumbv7m-none-eabi`. The `mime` and `std` features need `std`. No `unsafe`.

## Minimum supported Rust version

Rust 1.88.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for
inclusion in this crate by you, as defined in the Apache-2.0 license, shall be dual
licensed as above, without any additional terms or conditions.

## Credit

- Calendar arithmetic uses Howard Hinnant's
  [`days_from_civil` and `civil_from_days`](https://howardhinnant.github.io/date_algorithms.html).
- Time zones come from [jiff](https://crates.io/crates/jiff) and its bundled copy of the
  [IANA time-zone database](https://www.iana.org/time-zones).
- Cron semantics follow Paul Vixie's cron; [croner](https://crates.io/crates/croner)
  and [chrono-tz](https://crates.io/crates/chrono-tz) serve as test oracles.
