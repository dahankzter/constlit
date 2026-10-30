# constlit Phase 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship constlit's workspace, its error infrastructure and its first five macros: `dur!`, `size!`, `ip!`, `addr!` and `cidr!`. Each turns a string literal into a typed constant, or into a precise compile error.

**Architecture:** There are two crates.
- `constlit-macros` (proc macro) holds one pure parser module per macro, `parse_*(&str) -> Result<Value, String>`, which is unit-tested as ordinary code, plus a token emitter per macro.
- `constlit` (`no_std`) holds the own types (`ByteSize`, `Ipv4Cidr`, `Ipv6Cidr`, `Cidr`) and re-exports the macros.

Generated code uses only `::core::…` and `::constlit::…` paths, and only `const fn` constructors, so every macro works in `const` items.

**Tech Stack:** Rust 1.88+, edition 2024. `proc-macro2` 1, `quote` 1, `litrs` 1 (no `syn`). `trybuild` for compile-fail tests. `ipnet` 2 as an optional feature, and as a dev-dependency used as the test oracle.

**Spec:** `docs/superpowers/specs/2026-10-30-constlit-design.md` (macros 1-5, Errors, Own types, Testing, Constraints)

## Global Constraints

- Stable Rust, minimum 1.88, edition 2024.
- `constlit` is `#![no_std]` with no allocation and no required dependencies.
- `constlit-macros` depends on `proc-macro2`, `quote` and `litrs` (with its `proc-macro2` feature) only. `jiff` arrives in phase 3.
- No `unsafe` anywhere.
- License `MIT OR Apache-2.0`; repository `https://github.com/dahankzter/constlit`.
- Every macro's expansion must be usable in a `const` item.
- Every error is a `compile_error!` spanned on the literal, and names the problem plus the nearest valid literal when one exists.
- No silent ambiguity: when a literal could mean two things, it is an error with a suggestion.
- Own types have private fields, `const fn` accessors, a `#[doc(hidden)] pub const fn __new_unchecked(..)` for the macros, and checked public constructors.
- Whitespace rule (decided here; the spec allows "optional spaces"): single spaces are allowed between a number and its unit and between components. Leading or trailing whitespace, and anything other than a single ASCII space, is an error: `remove the spaces around the literal`.

## Review Focus

1. **Decimal edge forms:** `".5s"`, `"1.s"`, `"1..5s"` must be errors suggesting `"0.5s"` or `"1s"`, never a silent value. Pinned by `dur_decimal_edges` (Task 2).
2. **Leading or trailing whitespace and doubled spaces:** `" 1h"`, `"1h  30m"` and `"64 KiB "` must be errors. Pinned by `whitespace_rules` (Tasks 2 and 3).
3. **u64 edges for sizes:** `"18446744073709551615"` is OK, `"18446744073709551616"` and `"16384 PiB"` are overflow errors. Pinned by `size_u64_edges` (Task 3).
4. **IPv4 with leading zeros (`"010.0.0.1"`)** must be an error explaining the ambiguity (std rejects it too), not a silent decimal or octal reading. Pinned by `ipv4_leading_zeros` (Task 4).
5. **CIDR corners:**
   - `/0` and `/32` (v4), `/0` and `/128` (v6) are valid;
   - `0.0.0.0/0` contains every address;
   - IPv6 host bits are rejected with a correct suggestion.

   Pinned by `cidr_corners` (Task 5).

---

### Task 1: Workspace, crates and literal/error infrastructure

**Files:**
- Create: `Cargo.toml`
  - workspace with members `constlit`, `constlit-macros`; `resolver = "3"`;
  - `[workspace.package]` with version `0.1.0`, edition `2024`, rust-version `1.88`, license `MIT OR Apache-2.0`, repository `https://github.com/dahankzter/constlit`.
- Create: `constlit/Cargo.toml` (dep `constlit-macros = { path = "../constlit-macros", version = "=0.1.0" }`; dev-dep `trybuild = "1"`), `constlit/src/lib.rs`
- Create: `constlit-macros/Cargo.toml` (`proc-macro = true`; deps `proc-macro2 = "1"`, `quote = "1"`, `litrs = { version = "1", features = ["proc-macro2"] }`), `constlit-macros/src/lib.rs`, `constlit-macros/src/input.rs`
- Test: unit tests in `input.rs`

**Interfaces:**
- Produces, in `constlit-macros/src/input.rs`:
  - `pub struct Error { pub span: Span, pub message: String }` with `pub fn to_compile_error(&self) -> TokenStream`, which emits `::core::compile_error!(msg)` spanned at `span`.
  - `pub fn string_literal(input: TokenStream, example: &str) -> Result<(String, Span), Error>`. It accepts exactly one string literal, normal or raw, decoded with `litrs::StringLit`.
    - A non-literal or empty input gives `expected a string literal, like {example}`.
    - Extra tokens give `expected only one string literal`.
    - A suffixed literal gives `string literals can't have a suffix`.
  - `pub fn expand(input: TokenStream, example: &str, f: impl FnOnce(&str) -> Result<TokenStream, String>) -> TokenStream`. It runs `string_literal`, calls `f` on the text, and turns an `Err(message)` into a compile error spanned on the literal.
  - `pub fn check_spaces(s: &str) -> Result<(), String>`, the whitespace rule from Global Constraints, shared by `dur!` and `size!`.

- [ ] **Step 1: Write the failing tests** in `input.rs`:
  - `accepts_plain_and_raw`: `"\"1h\""` and `r#"r"1h""#` both give `"1h"`.
  - `rejects_non_literal_and_extra_tokens`: inputs `1`, `x` and `"a" "b"` give the messages above.
  - `rejects_suffix`: `"\"1h\"x"` gives the suffix message.
  - `space_rule`: `check_spaces` accepts `"1h 30m"`; it rejects `" 1h"`, `"1h "`, `"1h  30m"` and `"1h\t30m"` with `remove the spaces around the literal` or `use single spaces between parts`.
- [ ] **Step 2: Run** `cargo test -p constlit-macros`. Expected: FAIL (functions missing).
- [ ] **Step 3: Implement** the workspace, both crates and `input.rs`. `constlit/src/lib.rs` is `#![no_std]` with a crate doc line.
- [ ] **Step 4: Run** `cargo test -p constlit-macros && cargo build -p constlit --target thumbv7m-none-eabi`. Expected: tests PASS and the `no_std` build succeeds.
- [ ] **Step 5: Commit** `git commit -m "workspace: constlit and constlit-macros with literal and error helpers"`

---

### Task 2: `dur!`

**Files:**
- Create: `constlit-macros/src/duration.rs`
- Modify: `constlit-macros/src/lib.rs` (`#[proc_macro] pub fn dur`), `constlit/src/lib.rs` (`pub use constlit_macros::dur;`)
- Test: unit tests in `duration.rs`, `constlit/tests/dur.rs`, `constlit/tests/ui/dur_*.rs`, `constlit/tests/ui.rs`

**Interfaces:**
- Consumes: `input::{expand, check_spaces}` (Task 1).
- Produces:
  - `pub fn parse_duration(s: &str) -> Result<(u64, u32), String>` (seconds, nanoseconds);
  - `dur!` expands to `::core::time::Duration::new(secs, nanos)`.

**Grammar:**
- `component := number unit`. `number` is digits, optionally followed by `.` and more digits; both sides need at least one digit.
- Units, largest first: `d` = 86400 s, `h`, `m`, `s`, `ms`, `us`, `µs`, `ns`. Each unit appears at most once, in that order.
- Arithmetic is exact (integer nanoseconds as `u128`, then split). A decimal must give a whole number of nanoseconds.

Error messages (exact):

| input | message |
|---|---|
| `""` | `empty duration; write a number and a unit, like "30s"` |
| `"30"` | `missing unit after 30; write e.g. "30s" or "30ms"` |
| `"5hr"` | ``unknown unit `hr`; use `h` for hours`` (also: `hrs`/`hour`/`hours` → `h`, `min`/`mins`/`minute(s)` → `m`, `sec`/`secs`/`second(s)` → `s`, `msec` → `ms`, `day`/`days` → `d`) |
| `"5x"` | ``unknown unit `x`; valid units are ns, us, µs, ms, s, m, h, d`` |
| `"2w"`, `"1mo"`, `"1y"` | `weeks, months and years are not supported (months and years have no fixed length); write days instead, like "14d"` (also `wk`, `week(s)`, `month(s)`, `yr`, `year(s)`) |
| `"30m1h"`, `"1h2h"` | ``units must go from largest to smallest, each at most once: `h` after `m` `` (or `` `h` twice ``) |
| `"-5s"` | `durations can't be negative` |
| `".5s"` | `write a digit before the point, like "0.5s"` |
| `"1.s"` | `write a digit after the point, like "1s" or "1.5s"` |
| `"1.5ns"` | `1.5ns is not a whole number of nanoseconds` |
| `"999999999999999999999d"` | `duration too large for Duration` |

- [ ] **Step 1: Write the failing tests.**
  - In `duration.rs`:
    - `dur_values`:
      - `"1h30m"` → `(5400, 0)`
      - `"1h 30m"` → `(5400, 0)`
      - `"1.5h"` → `(5400, 0)`
      - `"500ms"` → `(0, 500_000_000)`
      - `"2m30s"` → `(150, 0)`
      - `"1d 12h"` → `(129600, 0)`
      - `"250us"` and `"250µs"` → `(0, 250_000)`
      - `"7ns"` → `(0, 7)`
      - `"0s"` → `(0, 0)`
      - `"0.5s"` → `(0, 500_000_000)`
    - `dur_errors`: every row of the table.
    - `dur_decimal_edges`: `".5s"`, `"1.s"` and `"1..5s"` are errors.
    - `whitespace_rules`: `" 1h"`, `"1h  30m"`.
    - `dur_round_trip`: for 2,000 SplitMix64-random durations, format as `"{d}d{h}h{m}m{s}s{ns}ns"` with zero components omitted; parsing gives back the same (secs, nanos). Use a tiny local SplitMix64; no RNG dependency.
  - In `constlit/tests/dur.rs`:
    - `dur_in_const`: `const T: Duration = constlit::dur!("1h30m"); assert_eq!(T, Duration::from_secs(5400));`
  - In `constlit/tests/ui/`:
    - `dur_bare_number.rs` (`dur!("30")`)
    - `dur_unknown_unit.rs` (`dur!("5hr")`)
    - `dur_months.rs` (`dur!("1mo")`)
- [ ] **Step 2: Run** `cargo test -p constlit-macros duration`. Expected: FAIL.
- [ ] **Step 3: Implement** `parse_duration` and the `dur` macro, with a hand-written left-to-right scanner.
- [ ] **Step 4: Run** `cargo test -p constlit-macros && cargo test -p constlit --test dur`, then `TRYBUILD=overwrite cargo test -p constlit --test ui` and review each `.stderr`: the caret must sit on the literal and show the exact message. Expected: PASS.
- [ ] **Step 5: Commit** `git commit -m "dur!: durations like \"1h30m\" as core::time::Duration"`

---

### Task 3: `ByteSize` and `size!`

**Files:**
- Create: `constlit/src/size.rs`, `constlit-macros/src/size.rs`
- Modify: both `lib.rs` files
- Test: unit tests in both `size.rs` files, `constlit/tests/size.rs`, `constlit/tests/ui/size_*.rs`

**Interfaces:**
- Consumes: Task 1 helpers.
- Produces:
  - `constlit::ByteSize`: `#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]`, private `u64`.
    - `pub const fn from_bytes(bytes: u64) -> Self` (every `u64` is valid, so no unchecked constructor is needed);
    - `pub const fn bytes(self) -> u64`;
    - `From<ByteSize> for u64`;
    - `Display`.
  - `pub fn parse_size(s: &str) -> Result<u64, String>`; `size!` expands to `::constlit::ByteSize::from_bytes(n)`.

**`Display` (decided):**
- IEC units with the largest unit whose value is at least 1, up to two decimals with trailing zeros trimmed:
  - `65536` → `64 KiB`
  - `1536` → `1.5 KiB`
  - `1000` → `1000 B`
  - `0` → `0 B`
- Exact values print exactly. Inexact ones round to two decimals and print with a `~` prefix: `1100` → `~1.07 KiB`.

**Units:**
- `B` or none = 1.
- `KB` `MB` `GB` `TB` `PB` = 1000^1..5.
- `KiB` `MiB` `GiB` `TiB` `PiB` and `Ki` `Mi` `Gi` `Ti` `Pi` = 1024^1..5.
- Case-sensitive. An optional single space between number and unit. Decimals must give whole bytes.

Error messages (exact):
- **Wrong case** (any spelling whose lowercase is `kb`, `kib`, `ki`, and so on, but isn't an exact unit):

  `` `{u}` is ambiguous: write `{IEC}` ({1024^k} bytes) or `{SI}` ({1000^k} bytes) ``

  For example `` `kb` is ambiguous: write `KiB` (1024 bytes) or `KB` (1000 bytes) ``.
- **Unknown unit:** ``unknown unit `{u}`; use B, KB/MB/GB/TB/PB (powers of 1000) or KiB/MiB/GiB/TiB/PiB (powers of 1024)``
- **Not whole bytes:** `{lit} is not a whole number of bytes`
- **Overflow:** `size too large for u64`
- **Negative:** `sizes can't be negative`
- **Empty:** `empty size; write a number and a unit, like "64 KiB"`

- [ ] **Step 1: Write the failing tests.**
  - `size_values`:
    - `"64 KiB"` and `"64KiB"` → 65536
    - `"64 KB"` → 64000
    - `"1.5 KiB"` → 1536
    - `"128Mi"` → 134217728
    - `"4096"` → 4096
    - `"4096 B"` → 4096
    - `"1 PiB"` → 1125899906842624
  - `size_errors`:
    - `"64 kb"` → the `kb` ambiguity message;
    - `"1 mb"` → `` `mb` is ambiguous: write `MiB` (1048576 bytes) or `MB` (1000000 bytes) ``;
    - `"1 KIB"` → ambiguous;
    - `"1 XB"` → unknown unit;
    - `"1.3 B"` → not whole;
    - `"-1 KiB"` → negative.
  - `size_u64_edges`: `"18446744073709551615"` is OK; `"18446744073709551616"` and `"16384 PiB"` overflow.
  - `whitespace_rules`: `"64 KiB "` and `"64  KiB"` are errors.
  - `display` (in `constlit/src/size.rs`): the four exact examples and the `~` case.
  - `size_in_const` (`constlit/tests/size.rs`): `const L: ByteSize = size!("64 KiB");` and `L.bytes() == 65536`.
  - UI files: `size_ambiguous.rs` (`size!("64 kb")`), `size_unknown.rs`.
- [ ] **Step 2: Run** `cargo test -p constlit-macros size && cargo test -p constlit`. Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the same commands and the UI snapshots, as in Task 2. Expected: PASS.
- [ ] **Step 5: Commit** `git commit -m "size!: strict byte sizes as constlit::ByteSize"`

---

### Task 4: `ip!` and `addr!`

**Files:**
- Create: `constlit-macros/src/net.rs`
- Modify: both `lib.rs` files
- Test: unit tests in `net.rs`, `constlit/tests/net.rs`, `constlit/tests/ui/ip_*.rs`, `constlit/tests/ui/addr_*.rs`

**Interfaces:**
- Consumes: Task 1 helpers.
- Produces:
  - `pub enum Ip { V4([u8; 4]), V6([u16; 8]) }` and `pub fn parse_ip(s: &str) -> Result<Ip, String>`. This uses `std::net::Ipv4Addr`/`Ipv6Addr` parsing at build time, with clearer errors layered on top.
  - `pub fn parse_socket_addr(s: &str) -> Result<(Ip, u16), String>`.
  - Expansions:
    - `ip!` → `::core::net::Ipv4Addr::new(a, b, c, d)` or `::core::net::Ipv6Addr::new(s0, …, s7)`;
    - `addr!` → `::core::net::SocketAddrV4::new(<v4>, port)` or `::core::net::SocketAddrV6::new(<v6>, port, 0, 0)`.

Error messages (exact):
- **Zone suffix:** `zone suffixes like %eth0 can't be stored in Ipv6Addr; remove it`
- **Leading zeros:** `IPv4 parts can't have leading zeros (is "010" decimal or octal?); write 10.0.0.1`. The suggestion is the input with leading zeros removed.
- **Not an address:** `` `{s}` is not an IPv4 or IPv6 address ``
- **Hostname:** `` `{host}` is a hostname; addr! needs an IP address (looking up a name isn't a literal) ``. A non-IP host is a hostname when it matches DNS label characters.
- **IPv6 without brackets:** `IPv6 socket addresses need brackets, like "[::1]:443"`
- **Missing port:** `missing port; write e.g. "{ip}:8080"`
- **Port out of range:** `port {p} is out of range (0-65535)`

- [ ] **Step 1: Write the failing tests.**
  - `ip_values`:
    - `"10.0.0.1"` → V4 `[10, 0, 0, 1]`;
    - `"::1"` → V6 `[0, 0, 0, 0, 0, 0, 0, 1]`;
    - `"::ffff:10.0.0.1"` → V6 `[0, 0, 0, 0, 0, 0xffff, 0x0a00, 0x0001]`.
  - `ip_errors`: zone, garbage, and `"256.0.0.1"`.
  - `ipv4_leading_zeros`: `"010.0.0.1"` gives the leading-zero message with `10.0.0.1`.
  - `addr_values`: `"0.0.0.0:8080"` and `"[::1]:443"`.
  - `addr_errors`: `"localhost:80"`, `"::1:443"` (brackets), `"10.0.0.1"` (missing port), `"10.0.0.1:65536"`.
  - `net_types` (`constlit/tests/net.rs`):
    - `const A: Ipv4Addr = ip!("10.0.0.53");`
    - `const B: Ipv6Addr = ip!("::1");`
    - `const S: SocketAddrV6 = addr!("[::1]:443");`
    - each equals `std`'s parse of the same string.
  - UI files: `ip_zone.rs`, `addr_hostname.rs`, `addr_no_brackets.rs`.
- [ ] **Step 2: Run** `cargo test -p constlit-macros net`. Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the tests and the UI snapshots. Expected: PASS.
- [ ] **Step 5: Commit** `git commit -m "ip!, addr!: IP and socket addresses as core::net types"`

---

### Task 5: CIDR types and `cidr!`

**Files:**
- Create: `constlit/src/cidr.rs`, add `cidr` to `constlit-macros/src/net.rs`
- Modify: `constlit/Cargo.toml` (`[features] ipnet = ["dep:ipnet"]`; optional dep `ipnet = { version = "2", optional = true, default-features = false }`; dev-dep `ipnet = "2"`), both `lib.rs` files
- Test: unit tests in `cidr.rs`, `constlit/tests/cidr.rs`, `constlit/tests/ui/cidr_*.rs`

**Interfaces:**
- Consumes: `Ip`, `parse_ip` (Task 4).
- Produces:
  - `constlit::Ipv4Cidr` and `Ipv6Cidr` (derive `Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash`). Private fields `addr` and `prefix: u8`.
    - `pub const fn new(addr, prefix: u8) -> Result<Self, CidrError>`;
    - `#[doc(hidden)] pub const fn __new_unchecked(addr, prefix: u8) -> Self`;
    - `const fn` accessors `network()`, `prefix()`, `netmask()`;
    - `pub const fn contains(&self, ip: Ipv4Addr/Ipv6Addr) -> bool`;
    - `Ipv4Cidr::broadcast()`;
    - `Display` as `"10.0.0.0/8"`.
  - `pub enum CidrError { PrefixTooLong { max: u8 }, HostBitsSet }`, with `Display`.
  - `pub enum Cidr { V4(Ipv4Cidr), V6(Ipv6Cidr) }`, with `From` both ways and `contains(IpAddr) -> bool`. A v4 net never contains a v6 address, and vice versa.
  - With `ipnet`: `From<Ipv4Cidr> for ipnet::Ipv4Net` and `From<Ipv6Cidr> for ipnet::Ipv6Net`.
  - `pub fn parse_cidr(s: &str) -> Result<(Ip, u8), String>`; `cidr!` expands to `::constlit::Ipv4Cidr::__new_unchecked(::core::net::Ipv4Addr::new(..), p)` or its v6 counterpart.

Error messages (exact):
- **Missing prefix:** `missing /prefix; write e.g. "10.0.0.0/8"`
- **Prefix too long:** `prefix /{p} is too long for IPv4 (max 32)` (`IPv6 (max 128)` for v6)
- **Host bits set:** `host bits are set in {s}; did you mean "{network}/{p}"?`

- [ ] **Step 1: Write the failing tests.**
  - `cidr_values`: `"10.0.0.0/8"` and `"2001:db8::/32"`.
  - `cidr_errors`:
    - `"10.1.2.3/8"` → host bits, suggesting `"10.0.0.0/8"`;
    - `"2001:db8::1/32"` → host bits, suggesting `"2001:db8::/32"`;
    - `"10.0.0.0/33"`, `"10.0.0.0"`.
  - `cidr_corners`:
    - `0.0.0.0/0` contains `255.255.255.255`;
    - `10.0.0.1/32` contains only itself;
    - `::/0` and `::1/128` are valid;
    - `Ipv4Cidr::new(10.1.0.0, 8)` → `Err(HostBitsSet)`;
    - `broadcast()` of `10.0.0.0/8` is `10.255.255.255`.
  - `cidr_oracle` (`constlit/tests/cidr.rs`): for 2,000 random (address, prefix) pairs, `contains` agrees with `ipnet::Ipv4Net::contains`, and network, netmask and broadcast agree with `ipnet`. Same for v6 without broadcast.
  - `cidr_in_const`: `const NET: Ipv4Cidr = cidr!("10.0.0.0/8");`
  - UI file: `cidr_host_bits.rs`.
- [ ] **Step 2: Run** `cargo test -p constlit-macros cidr && cargo test -p constlit`. Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `cargo test --workspace && cargo test -p constlit --features ipnet && cargo build -p constlit --target thumbv7m-none-eabi`, then `cargo +1.88 test -p constlit`. Expected: all PASS.
- [ ] **Step 5: Commit** `git commit -m "cidr!: IPv4/IPv6 networks as constlit::Ipv4Cidr / Ipv6Cidr"`
