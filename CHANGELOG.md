# Changelog

## 0.2.0

Sixteen new macros for the systems programs deploy on, with the same strict,
compile-time checking. One goes at the crate root, the rest in four modules:

- `mac!` → `MacAddr`: MAC addresses with `:` or `-`.
- `unix::mode!` → `Mode`: octal, `ls` and absolute symbolic file modes.
- `unix::signal!` → `Signal`: signal names numbered for the build target (Linux
  generic, mips and sparc; macOS, iOS and the BSDs). A signal the target lacks is a
  compile error naming the platform.
- `linux::cap!`, `linux::caps!` → `Capability`, `CapSet`.
- `k8s::quantity!`, `k8s::cpu!`, `k8s::mem!` → `Quantity`, exact in milli-units.
  Values Kubernetes would round or cap are errors; `mem!("500m")` asks whether you
  meant `"500Mi"`.
- `k8s::dns_label!`, `dns1035_label!`, `dns_subdomain!`, `label_key!`, `label_value!`,
  `selector!` → checked `&'static str`.
- `oci::image!` → `ImageRef`, normalized like Docker (`"nginx"` is
  `docker.io/library/nginx:latest`); `oci::digest!` → `Digest`; `oci::platform!` →
  `Platform` with Go's names.
- New feature `k8s-openapi`: `From<k8s::Quantity>` for k8s-openapi's `Quantity`.
- The Kubernetes and container macros are tested against the Go code that defines
  those formats (apimachinery, distribution/reference, go-digest, containerd).
- "Did you mean" suggestions count a swap of neighbouring characters as one edit.

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
