# constlit

[![crates.io](https://img.shields.io/crates/v/constlit.svg)](https://crates.io/crates/constlit)
[![docs.rs](https://img.shields.io/docsrs/constlit)](https://docs.rs/constlit)
[![license](https://img.shields.io/crates/l/constlit.svg)](#license)

Readable string literals that become typed constants at compile time.

```rust
use constlit::k8s::{self, Quantity};
use constlit::oci::{self, ImageRef};
use constlit::{ByteSize, Timestamp, ZonedTimestamp, dur, ip, size, ts};
use std::net::Ipv4Addr;
use std::time::Duration;

const TIMEOUT: Duration = dur!("1h30m");
const LIMIT: ByteSize = size!("64 KiB");
const DNS: Ipv4Addr = ip!("10.0.0.53");
const DEADLINE: Timestamp = ts!("2026-10-30T14:00Z");
const LAUNCH: ZonedTimestamp = ts!("2026-10-30 14:00 Europe/Stockholm");
const MEMORY: Quantity = k8s::mem!("512Mi");
const IMAGE: ImageRef = oci::image!("nginx:1.27");

assert_eq!(TIMEOUT.as_secs(), 5400);
assert_eq!(LIMIT.bytes(), 65_536);
assert_eq!(LAUNCH.unix_seconds(), DEADLINE.unix_seconds() - 3600);
assert_eq!(MEMORY.value(), 512 << 20);
assert_eq!(IMAGE.canonical(), "docker.io/library/nginx:1.27");
```

Each macro parses its literal while your crate compiles:

- **Checked.** A malformed literal is a compile error that points at the string, says
  what is wrong, and suggests a valid literal.
- **Free at run time.** A valid literal becomes a constant of a real type. Nothing is
  parsed, allocated or looked up when the program runs, not even time zones.
- **Strict.** A literal that could mean two things is an error, never a guess: `"64 kb"`
  (1000 or 1024?), `"30"` (seconds?), a local time that happens twice when the clocks
  go back, a Kubernetes memory limit of `"500m"` (half a byte).
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
| `mac!` | `mac!("00:1a:2b:3c:4d:5e")` | `constlit::MacAddr` |
| `version!` | `version!("1.2.3-beta.1+build.5")` | `constlit::Version` |
| `version_req!` | `version_req!(">=1.2, <2")` | `constlit::VersionReq` |
| `mime!` | `mime!("text/html; charset=utf-8")` | `constlit::MediaType` |
| `cron!` | `cron!("*/5 9-17 * * MON-FRI")` | `constlit::CronSpec` |
| `date!` | `date!("2026-10-30")` | `constlit::Date` |
| `time!` | `time!("14:30:05.250")` | `constlit::TimeOfDay` |
| `ts!` | `ts!("2026-10-30T14:00+01:00")`, `ts!("2026-10-30 14:00 Europe/Stockholm")` | `constlit::Timestamp` or `ZonedTimestamp`, by literal |

Values from particular systems live in modules: `constlit::unix`, `constlit::linux`,
`constlit::k8s`, `constlit::oci`, `constlit::docker` and `constlit::systemd`. That keeps their type names (`Quantity`,
`Signal`, `Platform`, ...) clear of the ecosystem types you convert them to.

### Unix and Linux

| macro | example | value |
|---|---|---|
| `unix::mode!` | `mode!("0755")`, `mode!("rwxr-xr-x")`, `mode!("u=rwx,go=rx")` | `unix::Mode` |
| `unix::signal!` | `signal!("SIGTERM")`, `signal!("TERM")` | `unix::Signal`, numbered for the build target |
| `linux::cap!` | `cap!("CAP_NET_ADMIN")`, `cap!("NET_ADMIN")` | `linux::Capability` |
| `linux::caps!` | `caps!("CAP_NET_ADMIN, CAP_SYS_TIME")` | `linux::CapSet` |
| `linux::errno!` | `errno!("ENOENT")` | `linux::Errno`, numbered for the build target |
| `linux::ifname!` | `ifname!("enp3s0")` | `&'static str` |
| `linux::dev!` | `dev!("8:0")` | `linux::DevNum` |

### systemd

| macro | example | value |
|---|---|---|
| `systemd::timespan!` | `timespan!("5min 30s")` | `core::time::Duration` |
| `systemd::calendar!` | `calendar!("Mon..Fri 09:00")` | `systemd::CalendarSpec` |
| `systemd::unit!` | `unit!("getty@tty1.service")` | `&'static str` |

### Kubernetes

| macro | example | value |
|---|---|---|
| `k8s::quantity!` | `quantity!("1.5Gi")`, `quantity!("1e3")` | `k8s::Quantity` |
| `k8s::cpu!` | `cpu!("500m")` | `k8s::Quantity` |
| `k8s::mem!` | `mem!("512Mi")` | `k8s::Quantity` |
| `k8s::dns_label!`, `dns1035_label!`, `dns_subdomain!` | `dns_label!("kube-system")` | `&'static str` |
| `k8s::label_key!`, `label_value!` | `label_key!("app.kubernetes.io/name")` | `&'static str` |
| `k8s::selector!` | `selector!("app=web,env in (prod,staging)")` | `&'static str` |

### Containers

| macro | example | value |
|---|---|---|
| `oci::image!` | `image!("nginx")`, `image!("ghcr.io/org/app:1.2")` | `oci::ImageRef` |
| `oci::digest!` | `digest!("sha256:…")` | `oci::Digest` |
| `oci::platform!` | `platform!("linux/arm64/v8")` | `oci::Platform` |
| `docker::port!` | `port!("127.0.0.1:8080:80/tcp")` | `docker::PortMapping` |

Every macro works in `const` and `static` items. The crate's own types keep their
fields private, so every value is valid: there is no February 30 and no CIDR with
host bits set. Their accessors are `const fn`, and most have checked constructors
(`Date::new`, `Ipv4Cidr::new`, `Mode::new`, ...) for values computed at run time.

## Errors

The error sits on the literal and names the fix:

```text
error: "500m" is 0.5 bytes ("m" means milli); did you mean "500Mi" or "500M"?
 --> src/main.rs:2:33
  |
2 |     let _ = constlit::k8s::mem!("500m");
  |                                 ^^^^^^
```

A few more:

| literal | error |
|---|---|
| `size!("64 kb")` | ``kb` is ambiguous: write `KiB` (1024 bytes) or `KB` (1000 bytes)`` |
| `dur!("30")` | ``missing unit after 30; write e.g. "30s" or "30ms"`` |
| `cidr!("10.1.2.3/8")` | ``host bits are set in 10.1.2.3/8; did you mean "10.0.0.0/8"?`` |
| `version!("v1.2.3")` | ``remove the leading "v"; write "1.2.3"`` |
| `date!("2026-02-30")` | ``day 30 is out of range: February 2026 has 28 days`` |
| `ts!("2026-10-25 02:30 Europe/Stockholm")` | ``2026-10-25 02:30 happens twice in Europe/Stockholm; write "2026-10-25 02:30+02:00 Europe/Stockholm" (the first) or "2026-10-25 02:30+01:00 Europe/Stockholm" (the second)`` |
| `unix::mode!("u+x")` | ``"u+x" changes a mode; write an absolute mode like "u=rwx,go=rx" or "0755"`` |
| `unix::signal!("15")` | ``signal numbers differ between platforms; write the name, e.g. "SIGTERM" (15 on most Linux targets)`` |
| `k8s::dns_label!("My-App")` | ``names must be lower case; write "my-app"`` |
| `oci::image!("MyOrg/app")` | ``"MyOrg" would be read as a registry host; Docker Hub names are lower case: "myorg/app"`` |
| `oci::platform!("linux/x86_64")` | ``"x86_64" is not a Go architecture name; write "linux/amd64"`` |
| `docker::port!("8080:80/TCP")` | ``protocols are lower case: "8080:80/tcp"`` |
| `version_req!(">=1.2 <2")` | ``separate comparators with ",": ">=1.2, <2"`` |
| `systemd::timespan!("30")` | ``missing unit after 30; write e.g. "30s" or "30min"`` |

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
- **`mac!`:** six two-digit hex bytes with one separator, `:` or `-`, in either case.
- **`version!`:** [SemVer 2.0.0](https://semver.org) exactly, so `"v1.2.3"` and `"1.2"`
  are errors. `Ord`, `Eq` and `Hash` follow SemVer precedence and ignore build metadata.
- **`version_req!`:** Cargo's requirement syntax, as the `semver` crate parses it, and
  Cargo's matching, including its pre-release opt-in. Build metadata (which
  requirements ignore) is an error.
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
- **`unix::mode!`:** the permission bits as 3 or 4 octal digits, the 9-character `ls`
  form, or absolute symbolic clauses where unnamed classes get no bits (`"u=rwx"` is
  `0700`). Relative modes (`"u+x"`), Rust's `"0o755"` and an `ls` line with a file type
  are errors. `Display` is octal; `{:#}` is the `ls` form.
- **`unix::signal!`:** names with or without `SIG`, upper case, numbered for the build
  target: Linux (generic, mips and sparc numberings), macOS, iOS and the BSDs. A signal
  the target lacks is a compile error naming the platform; numbers and real-time
  signals are errors.
- **`linux::cap!`, `caps!`:** `CAP_X` or `X`, numbers 0-40; `"ALL"` and duplicates are
  errors.
- **`linux::errno!`:** names with their `E`, numbered for the build target (Linux and
  Android: generic, powerpc, mips and sparc); other targets are a compile error.
- **`linux::ifname!`, `dev!`:** the kernel's interface-name rule (printable ASCII, at
  most 15 bytes) and `major:minor` device numbers within its limits.
- **`docker::port!`:** the `docker run -p` form, with ranges, IPv6 in brackets and
  `tcp`/`udp`/`sctp`; port 0, leading zeros and other spellings Docker quietly accepts
  are errors with a suggestion.
- **`systemd::timespan!`, `calendar!`, `unit!`:** systemd's syntax. Time spans need a
  unit; `calendar!`'s `Display` is systemd's normalized form and `next_after` matches
  systemd's next elapse, also across DST changes; the only zone suffix is `UTC`.
- **`k8s::quantity!`, `cpu!`, `mem!`:** Kubernetes' quantity grammar, kept exactly in
  milli-units. Where Kubernetes would round or cap (finer than `1m`, beyond 2^63 - 1
  units) it is an error. `cpu!` rejects negatives and binary suffixes; `mem!` wants
  whole bytes and rejects `n`, `u` and `m`. `Display` keeps the literal as written.
- **`k8s` names, labels and `selector!`:** Kubernetes' own validation rules and
  label-selector syntax. An empty selector, which matches everything, is an error.
- **`oci::image!`:** Docker's reference grammar and normalization: `"nginx"` is
  `docker.io/library/nginx:latest`. Upper case in a repository, and a capitalized first
  part Docker would read as a host (`"MyOrg/app"`), are errors.
- **`oci::digest!`:** `sha256` or `sha512` with lower-case hex, decoded at compile time.
- **`oci::platform!`:** Go's `GOOS`/`GOARCH` names and the OCI variants; common other
  names (`x86_64`, `aarch64`, `armhf`, `macos`) are errors that suggest the Go name.

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
| `std` | | `Timestamp::to_system_time`, `unix::Mode::to_permissions` (on unix) |
| `jiff` | | `CronSpec::next_after` and `CalendarSpec::next_after` for jiff date-times; `to_jiff` on `Date`, `TimeOfDay` and `Timestamp`; `ZonedTimestamp::to_jiff_zoned` |
| `chrono` | | `CronSpec::next_after` and `CalendarSpec::next_after` for chrono date-times; `to_chrono` on `Date`, `TimeOfDay` and `Timestamp` |
| `ipnet` | | `From` the CIDR types into `ipnet::Ipv4Net` and `Ipv6Net` |
| `semver` | | `Version::to_semver`, `VersionReq::to_semver` |
| `mime` | | `MediaType::to_mime` (needs `std`) |
| `k8s-openapi` | | `From<k8s::Quantity>` for k8s-openapi's `Quantity`; your crate picks the Kubernetes version through k8s-openapi's `v1_*` features, as k8s-openapi requires |

## Platforms

`no_std` with no allocation; it builds for bare-metal targets such as
`thumbv7m-none-eabi`. The `mime`, `std` and `k8s-openapi` features need `std`. No
`unsafe`. `unix::signal!` needs a Linux, Android, macOS, iOS or BSD target and
`linux::errno!` a Linux or Android target; every other macro works everywhere.

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
- The Kubernetes, container and Docker macros are tested against the Go code that
  defines those formats: Kubernetes' `apimachinery`, Docker's `distribution/reference`
  and `go-connections`, `go-digest` and containerd's `platforms`, through fixtures
  generated by a small Go program in the repository. The systemd macros are tested
  against `systemd-analyze`, `version_req!` against the `semver` crate, signal and errno
  numbers against the `libc` crate, and capabilities against the `caps` crate.
