# Changelog

## 0.1.0

First release: eleven macros that turn string literals into typed constants at
compile time, with errors on the literal that suggest the fix.

- `dur!` → `core::time::Duration`; `size!` → `ByteSize` with strict IEC/SI units.
- `ip!`, `addr!`, `cidr!` → `core::net` addresses and `Ipv4Cidr`/`Ipv6Cidr`.
- `version!` → `Version` (SemVer 2.0.0); `mime!` → `MediaType`.
- `cron!` → `CronSpec` with `matches`, and `next_after` for jiff and chrono date-times
  following Vixie cron through daylight-saving changes (`catch_up_gaps()` to opt out
  of skipping).
- `date!`, `time!`, `ts!` → `Date`, `TimeOfDay`, `Timestamp`, and `ZonedTimestamp` for
  IANA zone names resolved at compile time from jiff's bundled database (`tz`
  feature, on by default).
- Features: `tz`, `std`, `jiff`, `chrono`, `ipnet`, `semver`, `mime`.
- `no_std`, no allocation, no `unsafe`; Rust 1.88.
