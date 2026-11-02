# constlit: design

Date: 2026-10-30
Status: draft for review

## Intent

Readable string literals in source that become real typed values at compile time, "as if by magic":

```rust
use constlit::{dur, size, ip, ts};

const TIMEOUT: Duration = dur!("1h30m");
const LIMIT: ByteSize = size!("64 KiB");
const DNS: Ipv4Addr = ip!("10.0.0.53");
const LAUNCH: Timestamp = ts!("2026-10-30 14:00 Europe/Stockholm");
```

Each macro parses and validates its literal while the crate compiles.
- A malformed literal is a compile error that points at the string, says what is wrong, and suggests a valid form.
- A valid one becomes a constant of a real type: no parsing, allocation or lookup at run time.

Why: values like these are usually written as strings that are parsed at run time (panicking or erroring late), or as raw numbers that hide their meaning (`5400` seconds, `65536` bytes). This crate gives the readability of the string and the guarantees and speed of a typed constant.

It is the second crate in the user's local `macroz` folder, after `pickmap`, and is meant for real use and publication on crates.io.

## Goals (version one)

Eleven macros, built in this order:

| # | macro | example | value |
|---|---|---|---|
| 1 | `dur!` | `dur!("1h30m")` | `core::time::Duration` |
| 2 | `size!` | `size!("64 KiB")` | `constlit::ByteSize` |
| 3 | `ip!` | `ip!("10.0.0.1")`, `ip!("::1")` | `core::net::Ipv4Addr` or `Ipv6Addr`, by literal |
| 4 | `addr!` | `addr!("0.0.0.0:8080")`, `addr!("[::1]:443")` | `core::net::SocketAddrV4` or `SocketAddrV6`, by literal |
| 5 | `cidr!` | `cidr!("10.0.0.0/8")` | `constlit::Ipv4Cidr` or `Ipv6Cidr`, by literal |
| 6 | `version!` | `version!("1.2.3-beta.1+build.5")` | `constlit::Version` |
| 7 | `mime!` | `mime!("text/html; charset=utf-8")` | `constlit::MediaType` |
| 8 | `cron!` | `cron!("*/5 * * * *")` | `constlit::CronSpec` |
| 9 | `date!` | `date!("2026-10-30")` | `constlit::Date` |
| 10 | `time!` | `time!("14:30:05.250")` | `constlit::TimeOfDay` |
| 11 | `ts!` | `ts!("2026-10-30T14:00:00Z")` | `constlit::Timestamp` or `ZonedTimestamp`, by literal |

Output types follow the "mix" the user chose:
- **std (`core`) types** wherever they can be built in a `const` context.
- **The crate's own small types** where std has nothing, or where the popular ecosystem type can't be built at compile time.
- **Opt-in conversions** to ecosystem types behind Cargo features.

Every macro must be usable in `const` and `static` initializers.

## Non-goals (version one; later candidates)

- Kubernetes values: resource quantities (`"500m"` CPU, `"128Mi"` memory), label selectors and similar. The user wants these later.
- Common Linux values: file modes (`"0755"`, `"rwxr-xr-x"`), signal names, and similar. Also later.
- Run-time parsing APIs. This crate is about literals; `FromStr` impls on the own types are optional extras, not a goal.
- Locale-specific formats (`10/30/2026`, `1,5h`).
- Durations in months or years, which have no fixed length; negative durations.

## Anti-goals

- **Silent ambiguity.** If a literal could reasonably mean two things, it is a compile error with a suggestion, never a silent pick. Examples:
  - `kb` is rejected (is it 1000 or 1024?);
  - a bare `"30"` duration is rejected (which unit?);
  - an ambiguous local time is rejected (which side of the daylight-saving change?);
  - host bits set in a CIDR are rejected.
- **Run-time cost.** No macro may expand to code that parses, allocates, or does a time-zone lookup at run time.
- **Heavy dependencies by default.** The runtime crate has no required dependencies. Ecosystem crates appear only behind opt-in features.
- **Unhelpful errors.** A bare "invalid literal" error is a bug.

## Constraints

- Stable Rust. Minimum Rust version 1.88, matching `pickmap`, unless a dependency forces a newer one (record the reason).
- The runtime crate `constlit` is `#![no_std]` and uses no allocation. Conversions to `std::time::SystemTime` sit behind an `std` feature.
- The proc-macro crate's dependencies stay light: `proc-macro2`, `quote`, `litrs` (no `syn`). `jiff` with its bundled time-zone database is used only at build time, inside the proc macro, and only for `ts!` with named zones.
- **Feature `tz` (my call):** `jiff` sits behind a default-on `tz` feature in `constlit`, forwarded to `constlit-macros`.
  - With `default-features = false`, a named zone in `ts!` is a compile error saying to enable `tz`.
  - Fixed-offset and UTC `ts!` work without it, through a hand-written calendar computation.
  - A time-zone database update ships as a patch release that bumps `jiff`.
- No `unsafe`.
- License: MIT OR Apache-2.0, like `pickmap`. The repository will be `dahankzter/constlit`.

## Crate layout (my call)

Two crates in one workspace:

1. **`constlit`** (runtime, `no_std`). The own types and their methods, the optional conversions, and re-exports of all macros. Users depend on this crate only.
2. **`constlit-macros`** (proc macro). One parser module per macro, unit-tested as ordinary code, plus the token emission. Generated code refers to `::constlit::...` and `::core::...` paths only.

There is no dependency cycle: the macros crate emits paths and does not depend on `constlit`.

## Own types: private fields (my call)

All own types have private fields.
- Read access is through `const fn` accessors.
- Macros build values through a `#[doc(hidden)] pub const fn __new_unchecked(..)` that is not public API.
- Checked run-time constructors return `Option` or `Result`: `Date::new(y, m, d)`, `Ipv4Cidr::new(addr, prefix)`, `Version::new(major, minor, patch)`, and so on.

This keeps every value valid: no February 30 and no host bits in a CIDR. `Ord`, `Display` and `contains` can rely on that. Fields can be made public later without a breaking change; the reverse would be breaking.

## The macros (decided)

### 1. `dur!` → `core::time::Duration`

- **Units:** `ns`, `us`, `µs`, `ms`, `s`, `m`, `h`, `d` (a day is exactly 24 hours).
- **Combining:** components may be combined, largest unit first, with optional spaces: `"1h30m"`, `"1h 30m"`, `"2m30s"`, `"1d 12h"`.
- **Decimals:** allowed (`"1.5h"`, `"0.5s"`) when the result is a whole number of nanoseconds; otherwise a compile error.
- **Errors:**
  - a bare number such as `"30"`, with a suggestion like `"30s"`;
  - an unknown unit;
  - a repeated unit (`"1h2h"`), or units out of order;
  - overflow beyond `Duration`'s range;
  - weeks, months and years, explaining why.
- **Expansion:** `::core::time::Duration::new(secs, nanos)`.

### 2. `size!` → `constlit::ByteSize`

- **Strict units:**
  - `KiB`, `MiB`, `GiB`, `TiB`, `PiB` (and Kubernetes' `Ki`, `Mi`, `Gi`, `Ti`, `Pi`) are powers of 1024;
  - `KB`, `MB`, `GB`, `TB`, `PB` are powers of 1000;
  - `B` is bytes, and a plain number (`"4096"`) is bytes.
- **Errors:** any other spelling (`kb`, `Kb`, `KIB`, `kB`) is a compile error suggesting the two strict readings, e.g. "did you mean `KiB` (1024) or `KB` (1000)?".
- **Space:** an optional space between number and unit.
- **Decimals:** allowed when the result is a whole number of bytes (`"1.5 KiB"` is 1536).
- **Range:** values must fit in `u64`.
- **Type:** `ByteSize(u64)`, with const accessors (`bytes()`, `kib()` and so on as floats or truncating integers; builder's call on the exact set), `Display` choosing a readable unit, and `Ord`.

### 3. `ip!` → `Ipv4Addr` or `Ipv6Addr`

The literal decides the type:
- dotted IPv4 → `core::net::Ipv4Addr`;
- IPv6 in any standard textual form (including `::`, embedded IPv4 `::ffff:10.0.0.1`) → `core::net::Ipv6Addr`.

Zone or scope suffixes (`fe80::1%eth0`) are compile errors, since `Ipv6Addr` cannot hold them. Users who need `IpAddr` call `.into()`.

**My call:** returning the specific type rather than `IpAddr` keeps it more useful (`Ipv4Addr` methods work directly).

### 4. `addr!` → `SocketAddrV4` or `SocketAddrV6`

- `"host:port"` for IPv4 and `"[v6]:port"` for IPv6; brackets are required for IPv6.
- Port `0..=65535`.
- Hostnames are compile errors: they need DNS, which is not a literal.
- Expands to `SocketAddrV4::new(..)` or `SocketAddrV6::new(.., 0, 0)`.

### 5. `cidr!` → `constlit::Ipv4Cidr` or `Ipv6Cidr`

- `"addr/prefix"`, with the prefix in `0..=32` (v4) or `0..=128` (v6).
- **Host bits must be zero:** `"10.1.2.3/8"` is a compile error suggesting `"10.0.0.0/8"`.
- **Types:** both are const-constructible structs (`addr`, `prefix`) with `contains(addr)`, `network()`, `broadcast()` (v4), `hosts()`/`len()` (builder's call) and `Display`. An enum `Cidr` covers either, via `From`.
- **Feature `ipnet`:** `From` into `ipnet::Ipv4Net` and `Ipv6Net`.

### 6. `version!` → `constlit::Version`

- Strict SemVer 2.0.0: `MAJOR.MINOR.PATCH[-PRERELEASE][+BUILD]`.
- **Errors:** leading zeros in numeric identifiers, empty identifiers and missing components (`"1.2"`), each with a specific message.
- **Type:** `Version { major: u64, minor: u64, patch: u64, pre: &'static str, build: &'static str }`.
  - `Ord` follows SemVer precedence: pre-release identifiers compared field by field, numeric below alphanumeric, build ignored.
  - `Eq` is consistent with that `Ord`.
  - `Display` gives back the canonical form.
- **Feature `semver`:** `to_semver(&self) -> semver::Version` (run-time construction of their type; cannot fail for a validated literal).

### 7. `mime!` → `constlit::MediaType`

- `type/subtype` plus optional `; name=value` parameters, following RFC 6838 / RFC 9110 token rules (quoted parameter values allowed).
- Type, subtype and parameter names are normalized to lowercase; parameter values keep their case, except `charset`, whose value is lowercased.
- **Type:** `MediaType { essence: &'static str, slash: usize, params: &'static [(&'static str, &'static str)] }` with `type_()`, `subtype()`, `essence()`, `param(name)` and `Display`.
- **Feature `mime`:** `to_mime(&self) -> mime::Mime`.

### 8. `cron!` → `constlit::CronSpec`

- **Syntax:**
  - standard 5 fields (minute, hour, day of month, month, day of week);
  - lists, ranges and steps (`1-5`, `*/15`, `1,15,30`);
  - month and day names (`JAN`..`DEC`, `SUN`..`SAT`, case-insensitive);
  - day of week `0`-`7`, where `0` and `7` are both Sunday;
  - shortcuts `@yearly`/`@annually`, `@monthly`, `@weekly`, `@daily`/`@midnight`, `@hourly`.
- **Errors:** `@reboot` is a compile error (not a schedule). So are out-of-range values and empty ranges.
- **Type:** `CronSpec` holds the original string (`as_str()`) and decoded bitmasks:
  - minutes `u64`, hours `u32`, days of month `u32`, months `u16`, days of week `u8`;
  - flags recording whether day-of-month and day-of-week were restricted (`*` versus explicit).
- **Matching:** `matches(minute, hour, day, month, weekday) -> bool` is built in. It uses standard (Vixie) cron semantics: if both day fields are restricted, a time matches when either matches.
- **Restricted (my call, following Vixie exactly):** a day field is unrestricted when it starts with `*`, including `*/2`, and restricted otherwise. So `0 0 */2 * MON` fires on odd days that are Mondays. Oracle tests use `croner` in its Vixie-compatible mode; the `cron` crate combines the day fields with AND and can't serve as the oracle here.
- **Features `jiff` and `chrono`:** `next_after(&self, t) -> Option<...>` for those crates' civil or zoned date-times.
- **Daylight-saving changes in `next_after` (my call, documented on the method):** matching happens on local wall-clock time in the value's zone.
  - **Skipped local time** (spring-forward gap): a job scheduled inside the gap runs once, at the first instant after the gap.
  - **Repeated local time** (autumn overlap): a job with fixed minute and hour fields runs once, at the first occurrence. A job whose minute or hour field starts with `*` (including `*/5` and `@hourly`) runs on real time, as in Vixie cron, so it also fires through the second pass of the repeated hour (the user's decision, 2026-11-02).
  - **`None`** is returned only when no matching time exists within the date-time type's range (for example `0 0 30 2 *`, February 30).
- `as_str()` lets the value be handed to schedulers that take strings.

### 9. `date!` → `constlit::Date`

- `YYYY-MM-DD` (ISO 8601 calendar date), years `0001`-`9999`.
- Impossible dates (`2026-02-30`, `2025-02-29`) are compile errors naming the month's length.
- **Type:** `Date { year: i16, month: u8, day: u8 }` with `weekday()`, `day_of_year()`, `Ord`, `Display`, as const fns where possible.
- **Features (my call):** `jiff` (to `jiff::civil::Date`) and `chrono` (to `chrono::NaiveDate`).

### 10. `time!` → `constlit::TimeOfDay`

- `HH:MM`, `HH:MM:SS` or `HH:MM:SS.fraction` (up to 9 fractional digits), 24-hour clock.
- `24:00` and leap seconds (`:60`) are compile errors.
- **Type:** `TimeOfDay { hour, minute, second, nanosecond }`.
- **Features (my call):** `jiff` (to `jiff::civil::Time`) and `chrono` (to `chrono::NaiveTime`).

### 11. `ts!` → `constlit::Timestamp` or `ZonedTimestamp`

- **Forms:**
  - `YYYY-MM-DD[T| ]HH:MM[:SS[.fraction]]` followed by `Z`, a fixed offset (`+02:00`, `-0530`), or a space and an IANA zone name (`Europe/Stockholm`);
  - a zone name may also follow an explicit offset (`2026-10-25 02:30+01:00 Europe/Stockholm`) to disambiguate.
- **Resolution:** everything is resolved at compile time. Named zones use the IANA time-zone database bundled with `jiff` inside the proc macro.
- **Gaps:** a local time that doesn't exist in that zone (spring-forward gap) is a compile error showing the gap.
- **Overlaps:** a local time that happens twice (autumn overlap) is a compile error unless an explicit offset picks one. The message shows both valid spellings.
- **Mismatched offset:** an explicit offset that is wrong for the zone at that instant is a compile error.
- **Types:**
  - `Timestamp { unix_seconds: i64, nanosecond: u32, offset_seconds: i32 }` for `Z`/offset forms;
  - `ZonedTimestamp { timestamp: Timestamp, zone: &'static str }` for named zones.

  Both have `unix_seconds()`, `date()`, `time()` and `Display`.
- **Features:** `jiff` (to `jiff::Timestamp`, and to `jiff::Zoned` for zoned values, resolving the zone with the run-time tz database), `chrono` (to `chrono::DateTime<FixedOffset>` and `DateTime<Utc>`), `std` (to `SystemTime`).
- **Documented caveat:** for future instants in named zones, the result uses the time-zone rules known at compile time. A later law change takes effect only after a rebuild with a newer `constlit`. The README says so plainly, and names the tzdb version in the crate docs.

## Errors

- Every error is a `compile_error!` spanned on the literal.
- Every message names the problem and, where one exists, the nearest valid literal. For example: ``unknown unit `hr`; use `h` for hours``, or ```kb` is ambiguous: write `KiB` (1024 bytes) or `KB` (1000 bytes)``.

## Testing

- **Parser unit tests** per macro: tables of valid inputs with expected values, and invalid inputs with expected messages.
- **Oracle tests** in dev-dependencies, comparing against established crates on generated inputs:
  - `core::net` parsing for `ip!`/`addr!`;
  - `ipnet` for `cidr!`;
  - `semver` for `version!` (including precedence on random version pairs);
  - `mime` for `mime!`;
  - the `cron` crate (or `croner`) for `cron!` (`next_after` agrees);
  - `jiff` for `date!`/`time!`/`ts!`, including random zoned times around DST transitions.
- **Compile-fail tests** (`trybuild`) for each macro's typical mistakes, checking that the message points at the literal.
- **`const` tests:** every macro used in a `const` item.
- **`no_std` build check** of the runtime crate (e.g. for `thumbv7m-none-eabi`).

## Docs and release

- README in the style of pickmap's: what it is, a table of all macros with examples, the strictness rules, the tz caveat, features, and minimum Rust version.
- Crate docs with doctested examples per macro.
- Released to crates.io by the user running the publish command (no blanket publish permissions); GitHub repo `dahankzter/constlit`.

## Phases (my call)

1. **Phase 1:** macros 1-5 (`dur!`, `size!`, `ip!`, `addr!`, `cidr!`), the workspace, error infrastructure and the test setup.
2. **Phase 2:** macros 6-8 (`version!`, `mime!`, `cron!`).
3. **Phase 3:** macros 9-11 (`date!`, `time!`, `ts!`, with the bundled tz database).
4. **Then:** README, crate docs, and the first release.
5. **Later:** Kubernetes and Linux values.

## Conversions in version one (my call)

The version-one conversion features are exactly the ones listed per macro above:
- `ipnet`, `semver`, `mime`, `jiff`, `chrono`, `std`;
- `ByteSize` has none.

Support for the `time` crate is a later candidate.

## Left to the builder

- Exact method sets on the own types beyond those named above.
- Internal parser structure (hand-written recursive descent is expected; no parser-combinator dependency).
- Whether `ByteSize`'s `Display` picks IEC or SI units (pick one and document it).
- Which date-time crate version the oracle tests pin.
