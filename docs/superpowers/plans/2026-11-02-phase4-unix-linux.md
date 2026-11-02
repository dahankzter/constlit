# constlit Phase 4 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `mac!` at the crate root, plus the `unix` module (`mode!`, `signal!`) and the `linux` module (`cap!`, `caps!`), with their own types and the module re-export pattern later phases reuse.

**Architecture:** This follows phases 1-3.
- `constlit-macros` gets one parser module per macro (`mac.rs`, `mode.rs`, `signal.rs`, `cap.rs`), each a pure `parse_*(&str) -> Result<_, String>`. Expansion goes through `input::expand`. The proc macros are exported under prefixed names (`unix_mode`, `linux_cap`, ...).
- `constlit` gets the modules `unix` and `linux`. Each re-exports its macros under short names (`pub use constlit_macros::unix_mode as mode;`).
- Signal numbers depend on the target, which a proc macro can't see. So `signal!` expands to an inline `const` block that reads a per-target table in the runtime crate (`constlit::unix::__signals`). A signal the target lacks becomes a const-evaluation panic, which is a compile error in the user's crate.

**Tech Stack:**
- As in phase 3.
- New dev-deps:
  - `libc = "0.2"` in both crates;
  - `caps = "0.5"` in `constlit` under `[target.'cfg(target_os = "linux")'.dev-dependencies]`.
- No new normal dependencies.

**Spec:** `docs/superpowers/specs/2026-11-02-constlit-0.2-design.md`. Relevant sections: Module layout, `mac!`, `unix::mode!`, `unix::signal!`, `linux::cap!`, `linux::caps!`, Constraints, Testing. Version-one rules come from `2026-10-30-constlit-design.md`.

**Starting point:** `main` at `03dfca8` (0.1.0 plus the 0.2 spec).
- `constlit-macros/src/input.rs` has `expand(input, example, parse)` and `check_spaces`.
- `constlit-macros/src/zone.rs` (only under `tz`) has `fn levenshtein` and the "did you mean" rule: distance ≤ 3 and `3 * distance <= written.len()`.
- The UI tests live in `constlit/tests/ui*/`, run by `constlit/tests/ui.rs`.
- Installed targets: `x86_64-unknown-linux-gnu` (host), `aarch64-unknown-linux-gnu`, `aarch64-apple-darwin`, `wasm32-unknown-unknown` and `thumbv7m-none-eabi`.

## Global Constraints

- Every earlier constraint still holds:
  - stable Rust ≥ 1.88, edition 2024;
  - `constlit` is `#![no_std]` with no allocation outside opt-in features;
  - no `unsafe`;
  - every error is a `compile_error!` on the literal, naming the problem and the nearest valid literal where one exists;
  - no silent ambiguity;
  - every macro works in `const` and `static` items;
  - leading or trailing whitespace is the error `remove the spaces around the literal`.
- Own types have private fields, `const fn` accessors, and a `#[doc(hidden)] pub const fn __new_unchecked(..)` where the type has invalid bit patterns.
- Generated code uses only `::constlit::…` and `::core::…` paths.
- The modules `unix` and `linux` are always compiled; there are no new features. `Mode::to_permissions` exists only with `feature = "std"` and `cfg(unix)`.
- Checks at the end of every task (same as phase 3, plus the targets):
  - `cargo test --workspace --all-features` and `cargo test --workspace`;
  - `cargo test -p constlit-macros` and `cargo test -p constlit-macros --features tz`;
  - `cargo clippy --workspace --all-targets -- -D warnings`, once with no features, once with `--all-features`, and once each with `--features jiff`, `--features chrono` and `--no-default-features`;
  - `cargo build -p constlit --target thumbv7m-none-eabi`;
  - `cargo +1.88 test --workspace --all-features`;
  - `cargo fmt --all --check`.

## Decisions this plan makes (my calls, within the spec)

- **The shared "did you mean" helper** moves from `zone.rs` to a new unconditional `constlit-macros/src/suggest.rs`, so `signal!` and `cap!` can use the same rule as zone names.
- **`Mode` display:** `{}` writes 4-digit octal (`0755`). The alternate flag `{:#}` writes the 9-character ls form (`rwxr-xr-x`), so no extra adaptor type is needed.
- **`Mode` bit queries:**
  - `is_setuid()`, `is_setgid()`, `is_sticky()`;
  - `user()`, `group()`, `other()`, each returning the class's `rwx` bits as `0..=7` (r=4, w=2, x=1).
- **The `signal!` expansion** (exact; `TERM` stands for the canonical name without `SIG`):
  ```rust
  const {
      if !::constlit::unix::__signals::SUPPORTED {
          ::core::panic!("signal! supports Linux, Android, macOS, iOS and the BSDs")
      }
      match ::constlit::unix::__signals::TERM {
          ::core::option::Option::Some(n) => ::constlit::unix::Signal::__new_unchecked(n),
          ::core::option::Option::None => ::core::panic!("SIGTERM does not exist on this target"),
      }
  }
  ```
  The spec asks for an error naming "the signal and the target family". Const panics can't format, so the message names the signal and says "this target". **Ruling to ledger:** cost if wrong is one message change.
- **Signal families**, as `cfg` predicates in `constlit/src/unix/signal.rs`:
  - `linux_generic`: `any(target_os = "linux", target_os = "android")` and not mips or sparc;
  - `linux_mips`: Linux or Android with `target_arch` `mips`, `mips64`, `mips32r6` or `mips64r6`;
  - `linux_sparc`: Linux with `target_arch` `sparc` or `sparc64`;
  - `bsd`: `target_vendor = "apple"`, or `target_os` `freebsd`, `netbsd`, `openbsd` or `dragonfly`;
  - anything else is unsupported: `SUPPORTED = false` and every entry `None`.
- **`Signal::name()`** returns the canonical name for the number on the build target. Aliases display canonically: `signal!("SIGIOT")` shows `SIGABRT`. On sparc, 29 is `SIGPWR` (`SIGLOST` is the same number).
- **The capability name table exists twice:** in `constlit-macros/src/cap.rs` for parsing and in `constlit/src/linux/cap.rs` for `name()`. The proc macro can't depend on the runtime crate. Each copy is tested against the `caps` crate.
- **`CapSet` display** joins names with `", "` in number order. The empty set displays as an empty string.

### Signal tables (exact; numbers 1-31 plus extras)

| name | generic | mips | sparc | bsd |
|---|---|---|---|---|
| HUP INT QUIT ILL TRAP ABRT | 1-6 | 1-6 | 1-6 | 1-6 |
| BUS | 7 | 10 | 10 | 10 |
| FPE | 8 | 8 | 8 | 8 |
| KILL | 9 | 9 | 9 | 9 |
| USR1 | 10 | 16 | 30 | 30 |
| SEGV | 11 | 11 | 11 | 11 |
| USR2 | 12 | 17 | 31 | 31 |
| PIPE | 13 | 13 | 13 | 13 |
| ALRM | 14 | 14 | 14 | 14 |
| TERM | 15 | 15 | 15 | 15 |
| STKFLT | 16 | – | – | – |
| CHLD | 17 | 18 | 20 | 20 |
| CONT | 18 | 25 | 19 | 19 |
| STOP | 19 | 23 | 17 | 17 |
| TSTP | 20 | 24 | 18 | 18 |
| TTIN | 21 | 26 | 21 | 21 |
| TTOU | 22 | 27 | 22 | 22 |
| URG | 23 | 21 | 16 | 16 |
| XCPU | 24 | 30 | 24 | 24 |
| XFSZ | 25 | 31 | 25 | 25 |
| VTALRM | 26 | 28 | 26 | 26 |
| PROF | 27 | 29 | 27 | 27 |
| WINCH | 28 | 20 | 28 | 28 |
| IO | 29 | 22 | 23 | 23 |
| PWR | 30 | 19 | 29 | – |
| SYS | 31 | 12 | 12 | 12 |
| EMT | – | 7 | 7 | 7 |
| INFO | – | – | – | 29 |
| LOST | – | – | 29 | – |

Aliases accepted by the macro: `IOT` → `ABRT`, `POLL` → `IO`, `CLD` → `CHLD`.

### Capability table (exact; numbers 0-40)

`CAP_CHOWN` 0, `CAP_DAC_OVERRIDE` 1, `CAP_DAC_READ_SEARCH` 2, `CAP_FOWNER` 3, `CAP_FSETID` 4, `CAP_KILL` 5, `CAP_SETGID` 6, `CAP_SETUID` 7, `CAP_SETPCAP` 8, `CAP_LINUX_IMMUTABLE` 9, `CAP_NET_BIND_SERVICE` 10, `CAP_NET_BROADCAST` 11, `CAP_NET_ADMIN` 12, `CAP_NET_RAW` 13, `CAP_IPC_LOCK` 14, `CAP_IPC_OWNER` 15, `CAP_SYS_MODULE` 16, `CAP_SYS_RAWIO` 17, `CAP_SYS_CHROOT` 18, `CAP_SYS_PTRACE` 19, `CAP_SYS_PACCT` 20, `CAP_SYS_ADMIN` 21, `CAP_SYS_BOOT` 22, `CAP_SYS_NICE` 23, `CAP_SYS_RESOURCE` 24, `CAP_SYS_TIME` 25, `CAP_SYS_TTY_CONFIG` 26, `CAP_MKNOD` 27, `CAP_LEASE` 28, `CAP_AUDIT_WRITE` 29, `CAP_AUDIT_CONTROL` 30, `CAP_SETFCAP` 31, `CAP_MAC_OVERRIDE` 32, `CAP_MAC_ADMIN` 33, `CAP_SYSLOG` 34, `CAP_WAKE_ALARM` 35, `CAP_BLOCK_SUSPEND` 36, `CAP_AUDIT_READ` 37, `CAP_PERFMON` 38, `CAP_BPF` 39, `CAP_CHECKPOINT_RESTORE` 40.

## Review Focus

1. **Absolute symbolic modes leave unmentioned classes at zero.**
   - `mode!("u=rwx")` is `0o700`, not "u changed, rest kept".
   - `mode!("a=")` is `0`.
   - `mode!("a=rwxst")` is `0o7777`.
   - Pinned by `mode_values` (Task 2).
2. **`signal!` outside `const` items.**
   - `let s = signal!("SIGTERM");` and a `static` must work.
   - A missing signal must still be a compile error in a `let`, which the inline `const` block forces.
   - Pinned by `signal_contexts` (Task 3) and the UI test `signal_missing.rs`, which uses `let`.
3. **Upper-case MAC addresses.** `mac!("FF:FF:FF:FF:FF:FF")` is accepted and displays as `ff:ff:ff:ff:ff:ff`; the broadcast address `is_multicast()`. Pinned by `mac_values` (Task 1).
4. **Two spellings of one capability in `caps!`.** `caps!("NET_ADMIN, CAP_NET_ADMIN")` is a duplicate error naming `CAP_NET_ADMIN`. The highest capability (40) round-trips, and `CapSet::from_bits(1 << 41)` is `None`. Pinned by `caps_errors` and `capset_ops` (Task 4).
5. **Aliases display canonically and compare equal.** `signal!("SIGIOT") == signal!("SIGABRT")`, and its `to_string()` is `"SIGABRT"`. Pinned by `signal_values` (Task 3).

---

### Task 1: `MacAddr` and `mac!`

**Files:**
- Create: `constlit/src/mac.rs`, `constlit-macros/src/mac.rs`
- Modify:
  - `constlit/src/lib.rs`: `mod mac; pub use mac::MacAddr;` and a documented `pub use constlit_macros::mac;` with a doctest;
  - `constlit-macros/src/lib.rs`: `#[proc_macro] pub fn mac`.
- Test: unit tests in `constlit-macros/src/mac.rs`; `constlit/tests/mac.rs`; `constlit/tests/ui/mac_mixed_separators.rs` and `mac_cisco.rs`.

**Interfaces:**
- Produces `constlit::MacAddr`:
  - private `[u8; 6]`; derives `Clone`, `Copy`, `Debug`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Hash`;
  - `pub const fn new(octets: [u8; 6]) -> MacAddr`;
  - `const fn` methods `octets() -> [u8; 6]`, `is_multicast()` (bit 0 of the first byte), `is_unicast()`, `is_local()` (bit 1 of the first byte), `is_universal()`;
  - `Display`: lowercase, colon-separated;
  - `From<[u8; 6]> for MacAddr` and `From<MacAddr> for [u8; 6]`.
- Produces `constlit_macros::mac::parse_mac(s: &str) -> Result<[u8; 6], String>`.
- `mac!` expands to `::constlit::MacAddr::new([0x00u8, 0x1au8, …])`.

Error messages (exact). The suggestions are lowercase, keep the separator as written, and default to `:` when there is none:

| input | message |
|---|---|
| `""` | `empty MAC address; write six hex bytes, like "00:1a:2b:3c:4d:5e"` |
| `"00:1a:2b-3c:4d:5e"` | `use one separator throughout: "00:1a:2b:3c:4d:5e"` (the first separator written) |
| `"001a.2b3c.4d5e"` | `write the address as six bytes: "00:1a:2b:3c:4d:5e"` |
| `"0:1a:2b:3c:4d:5e"` | `write each byte with two hex digits: "00:1a:2b:3c:4d:5e"` |
| `"001a2b3c4d5e"` | `add separators: "00:1a:2b:3c:4d:5e"` |
| `"00:1a:2b:3c:4d"` | `a MAC address has 6 bytes, found 5` (any count other than 6) |
| `"00:1a:2b:3c:4d:5g"` | ``byte `5g` is not hex`` |
| `" 00:1a:2b:3c:4d:5e"` | `remove the spaces around the literal` |

- [ ] **Step 1: Write the failing tests.**
  - `mac_values` (macros): `"00:1a:2b:3c:4d:5e"` and `"00-1A-2B-3C-4D-5E"` both → `[0x00, 0x1a, 0x2b, 0x3c, 0x4d, 0x5e]`; `"FF:FF:FF:FF:FF:FF"` → `[0xff; 6]`.
  - `mac_errors` (macros): every row of the table.
  - `mac_round_trip` (macros): 10,000 random `[u8; 6]` from SplitMix64, written colon-lowercase, dash-uppercase and mixed-case. `parse_mac` returns the bytes for all three spellings.
  - `mac_accessors` (runtime, `constlit/tests/mac.rs`):
    - `const M: MacAddr = mac!("02:00:5e:10:00:01");` gives `is_local()` and `is_unicast()`;
    - `mac!("01:00:5e:00:00:fb").is_multicast()`;
    - `mac!("FF:FF:FF:FF:FF:FF").to_string() == "ff:ff:ff:ff:ff:ff"` and it `is_multicast()`;
    - `MacAddr::new(M.octets()) == M`, plus `<[u8; 6]>::from(M)` and `MacAddr::from([0; 6])`;
    - `static S: MacAddr = mac!("00:00:00:00:00:01");`.
  - UI: `mac_mixed_separators.rs` (`mac!("00:1a:2b-3c:4d:5e")`) and `mac_cisco.rs` (`mac!("001a.2b3c.4d5e")`).
- [ ] **Step 2: Run** `cargo test -p constlit-macros mac && cargo test -p constlit --test mac`. Expected: FAIL (items missing).
- [ ] **Step 3: Implement.** Classify the shape first: count `:` and `-`, detect two dots, detect 12 bare hex digits. Then check byte widths, then hex.
- [ ] **Step 4: Run** the Global Constraints checks, then `TRYBUILD=overwrite cargo test -p constlit --test ui`. Read each new `.stderr`: the caret must be on the literal, with the exact message. Expected: PASS, no warnings.
- [ ] **Step 5: Commit** `git commit -m "mac!: MAC addresses as constlit::MacAddr"`

---

### Task 2: `unix` module, `Mode` and `unix::mode!`

**Files:**
- Create: `constlit/src/unix/mod.rs`, `constlit/src/unix/mode.rs`, `constlit-macros/src/mode.rs`
- Modify:
  - `constlit/src/lib.rs`: `pub mod unix;`;
  - `constlit-macros/src/lib.rs`: `#[proc_macro] pub fn unix_mode`.
- Test: unit tests in `constlit-macros/src/mode.rs` and `constlit/src/unix/mode.rs`; `constlit/tests/mode.rs`; `constlit/tests/ui/mode_relative.rs`, `mode_file_type.rs` and `mode_rust_octal.rs`.

**Interfaces:**
- Produces `constlit::unix` with:
  - module docs (one paragraph and an example);
  - `pub use mode::Mode;`;
  - the documented `pub use constlit_macros::unix_mode as mode;`, with doctests showing all three forms.

  **This is the re-export pattern for Tasks 3-4 and phases 5-6.**
- Produces `constlit::unix::Mode`:
  - private `u32`, always `<= 0o7777`; derives `Clone`, `Copy`, `Debug`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Hash`;
  - `pub const fn new(bits: u32) -> Option<Mode>` and `#[doc(hidden)] pub const fn __new_unchecked(bits: u32) -> Mode`;
  - `const fn` methods `bits() -> u32`, `is_setuid()`, `is_setgid()`, `is_sticky()`, `user() -> u8`, `group() -> u8`, `other() -> u8`;
  - `Display` as decided above (`{}` octal, `{:#}` ls form);
  - `From<Mode> for u32`;
  - `#[cfg(all(feature = "std", unix))] pub fn to_permissions(self) -> std::fs::Permissions`.
- Produces `constlit_macros::mode::parse_mode(s: &str) -> Result<u32, String>`.
- `unix::mode!` expands to `::constlit::unix::Mode::__new_unchecked(0o755u32)`.

The forms are octal, ls and absolute symbolic, as in the spec.

Error messages (exact):

| input | message |
|---|---|
| `""` | `empty mode; write octal like "0755", or "rwxr-xr-x"` |
| `"0o755"` | `write the mode without "0o": "0755"` |
| `"75"`, `"7"` | `write 3 or 4 octal digits, like "0755"` |
| `"00755"` | `write at most 4 octal digits: "0755"` (5+ digits whose value fits) |
| `"17777"` | `mode 17777 is out of range (the highest is 7777)` |
| `"0758"` | ``digit `8` is not octal (0-7)`` |
| `"-rwxr-xr-x"`, `"drwxr-xr-x"` | `remove the file type "d": "rwxr-xr-x"` (the type character as written) |
| `"rwxr-xr"` | `write all 9 permission characters, like "rwxr-xr-x"` |
| `"rwxr-xr-q"` | ``unexpected `q` at position 9; expected `x`, `t`, `T` or `-` `` (positions 1, 4 and 7 expect `` `r` or `-` ``; 2, 5 and 8 `` `w` or `-` ``; 3 and 6 `` `x`, `s`, `S` or `-` ``) |
| `"u+x"`, `"go-w"` | `"u+x" changes a mode; write an absolute mode like "u=rwx,go=rx" or "0755"` (the clause as written) |
| `"u=rwx,u=r"`, `"a=r,u=w"` | `"u" is set twice; set each of u, g and o once` |
| `"o=s"` | `"s" (setuid and setgid) applies only to u, g or a` |
| `"u=t"` | `"t" (sticky) applies only to o or a` |
| `"x=r"` | ``unknown class `x`; use u, g, o or a`` |
| `"u=rq"` | ``unknown permission `q`; use r, w, x, s or t`` |
| `"=r"` | `missing class before "="; write e.g. "a=r"` |
| `"u=rwx,"`, `"u=rwx,,g=r"` | `empty clause; remove the extra ","` |
| `"u=rwx, g=rx"` | `remove the spaces: "u=rwx,g=rx"` |

- [ ] **Step 1: Write the failing tests.**
  - `mode_values` (macros):
    - octal: `"755"` → `0o755`; `"0755"` → `0o755`; `"4755"` → `0o4755`; `"1777"` → `0o1777`; `"000"` → `0`;
    - ls form: `"rwxr-xr-x"` → `0o755`; `"rwsr-xr-x"` → `0o4755`; `"rwSr--r--"` → `0o4644`; `"rwxr-sr-x"` → `0o2755`; `"rwxrwxrwt"` → `0o1777`; `"rw-r--r-T"` → `0o1644`; `"---------"` → `0`;
    - symbolic: `"u=rwx,g=rx,o=rx"` → `0o755`; `"u=rwx,go=rx"` → `0o755`; `"u=rwx"` → `0o700`; `"a="` → `0`; `"a=rwxst"` → `0o7777`; `"u=rwxs,g=x"` → `0o4710`; `"o=t"` → `0o1000`; `"ug=rw,o=r"` → `0o664`.
  - `mode_errors` (macros): every row of the table.
  - `mode_forms_agree` (macros): for every `bits` in `0..=0o7777`, the 4-digit octal spelling, the ls spelling (built in the test) and a symbolic spelling (built in the test, e.g. `u=…,g=…,o=…` with `s` and `t` placed where they apply) all parse to `bits`.
  - `mode_api` (runtime unit test, `constlit/src/unix/mode.rs`):
    - `Mode::new(0o10000) == None`;
    - `Mode::new(0o755).unwrap().bits() == 0o755`.
  - `mode_accessors` (runtime, `constlit/tests/mode.rs`):
    - `const M: Mode = unix::mode!("rwsr-x---");` gives `bits() == 0o4750`, `is_setuid()`, `!is_setgid()`, `!is_sticky()`, `user() == 7`, `group() == 5`, `other() == 0`;
    - `M.to_string() == "4750"` and `format!("{M:#}") == "rwsr-x---"`;
    - `unix::mode!("644").to_string() == "0644"`;
    - `format!("{:#}", unix::mode!("1777")) == "rwxrwxrwt"`;
    - `u32::from(M) == 0o4750`;
    - with `std` on unix: `M.to_permissions().mode() & 0o7777 == 0o4750`.
  - UI: `mode_relative.rs` (`unix::mode!("u+x")`), `mode_file_type.rs` (`"-rwxr-xr-x"`) and `mode_rust_octal.rs` (`"0o755"`).
- [ ] **Step 2: Run** `cargo test -p constlit-macros mode && cargo test -p constlit --all-features`. Expected: FAIL (items missing).
- [ ] **Step 3: Implement.** Classification order:
  1. spaces;
  2. `0o` prefix;
  3. all digits gives octal;
  4. length 10 with the last 9 characters a valid ls form gives the file-type error. This is checked before symbolic, because `"-rwxr-xr-x"` starts with `-`;
  5. contains `=`, `+`, or a `-` that follows only class letters at a clause start, gives symbolic;
  6. otherwise the ls form.
- [ ] **Step 4: Run** the Global Constraints checks and `TRYBUILD=overwrite cargo test -p constlit --test ui`, then read the new `.stderr` files. Expected: PASS, no warnings.
- [ ] **Step 5: Commit** `git commit -m "unix::mode!: file permission modes as constlit::unix::Mode"`

---

### Task 3: `Signal` and `unix::signal!`

**Files:**
- Create: `constlit/src/unix/signal.rs`, `constlit-macros/src/signal.rs`, `constlit-macros/src/suggest.rs`
- Modify:
  - `constlit/src/unix/mod.rs`: `pub use signal::Signal;`, `#[doc(hidden)] pub use signal::table as __signals;` (or an equivalent hidden path), and the documented `pub use constlit_macros::unix_signal as signal;`;
  - `constlit-macros/src/zone.rs`: use `suggest::closest`, and delete its own `levenshtein`;
  - `constlit-macros/src/lib.rs`: `mod suggest;` (unconditional) and `#[proc_macro] pub fn unix_signal`;
  - `constlit/tests/ui.rs`: add `#[cfg(all(target_os = "linux", not(any(target_arch = "mips", target_arch = "mips64", target_arch = "sparc", target_arch = "sparc64"))))] t.compile_fail("tests/ui_linux/*.rs");`;
  - `constlit/Cargo.toml` and `constlit-macros/Cargo.toml`: dev-dep `libc = "0.2"`.
- Test:
  - unit tests in both new parser files and in `suggest.rs`;
  - `constlit/tests/signal.rs` and `constlit/tests/signal_libc.rs`;
  - UI: `constlit/tests/ui/signal_number.rs`, `signal_lowercase.rs` and `signal_realtime.rs`, plus `constlit/tests/ui_linux/signal_missing.rs`.

**Interfaces:**
- Produces `constlit_macros::suggest::closest<'a>(written: &str, candidates: impl IntoIterator<Item = &'a str>) -> Option<&'a str>`. It applies the existing rule (Levenshtein distance ≤ 3 and `3 * distance <= written.len()`, the nearest wins, and ties go to the first). Task 4 consumes it.
- Produces `constlit_macros::signal::parse_signal(s: &str) -> Result<&'static str, String>`. It returns the canonical name without `SIG` (`"TERM"`), with aliases resolved.
- Produces `constlit::unix::Signal`:
  - private `i32`; derives `Clone`, `Copy`, `Debug`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Hash`;
  - `#[doc(hidden)] pub const fn __new_unchecked(number: i32) -> Signal`;
  - `pub const fn from_number(number: i32) -> Option<Signal>`, which accepts only numbers in the target's table;
  - `const fn number() -> i32` and `const fn name() -> &'static str`;
  - `Display` writes `name()`.
- Produces the hidden table module:
  - `pub const SUPPORTED: bool`;
  - one `pub const <NAME>: Option<i32>` for each of `HUP INT QUIT ILL TRAP ABRT BUS FPE KILL USR1 SEGV USR2 PIPE ALRM TERM STKFLT CHLD CONT STOP TSTP TTIN TTOU URG XCPU XFSZ VTALRM PROF WINCH IO PWR SYS EMT INFO LOST`, with values from the signal table above for the target's family.
- The expansion is the exact block from "Decisions" above.

Error messages (exact):

| input | message |
|---|---|
| `""` | `empty signal; write a name like "SIGTERM"` |
| `"sigterm"`, `"Term"` | `signal names are upper case: "SIGTERM"` (the input upper-cased, keeping or omitting `SIG` as written: `"TERM"` for `"Term"`) |
| `"15"` | `signal numbers differ between platforms; write the name, e.g. "SIGTERM" (15 on most Linux targets)` (for 1-31, using the generic column) |
| `"0"`, `"64"`, `"-9"` | `signal numbers differ between platforms; write the name, like "SIGTERM"` |
| `"SIGRTMIN"`, `"SIGRTMAX"`, `"SIGRTMIN+3"`, `"RTMAX-1"` | `real-time signal numbers are only known at run time; use libc::SIGRTMIN() instead` |
| `"SIGTREM"` | `unknown signal "SIGTREM"; did you mean "SIGTERM"?` (suggestions are made among canonical names plus aliases, all with `SIG`) |
| `"SIGFOO"` | `unknown signal "SIGFOO"` |

- [ ] **Step 1: Write the failing tests.**
  - `suggest_rule` (`suggest.rs`):
    - `closest("Europe/Stokholm", ["Europe/Stockholm", "Europe/Oslo"]) == Some("Europe/Stockholm")`;
    - `closest("ab", ["xy"]) == None` (distance 2 > 2/3 of the length);
    - `closest("SIGTREM", names) == Some("SIGTERM")`.
  - The existing zone tests still pass after the move.
  - `signal_values` (macros):
    - `"SIGTERM"` and `"TERM"` → `"TERM"`;
    - `"SIGIOT"` → `"ABRT"`, `"SIGPOLL"` → `"IO"`, `"SIGCLD"` → `"CHLD"`;
    - all 34 names, with and without `SIG`, parse.
  - `signal_errors` (macros): every row of the table.
  - `signal_generic_oracle` (macros, `#[cfg(all(target_os = "linux", target_arch = "x86_64"))]`): the macros crate's generic-Linux number→name table (used for the "15 on most Linux targets" suggestion) matches `libc::SIG*` for 1-31.
  - `signal_values` (runtime, `constlit/tests/signal.rs`, host = generic Linux):
    - `const T: Signal = unix::signal!("SIGTERM");` gives `number() == 15` and `name() == "SIGTERM"`;
    - `unix::signal!("SIGIOT") == unix::signal!("SIGABRT")`, and its `to_string() == "SIGABRT"`;
    - `Signal::from_number(10).unwrap().name() == "SIGUSR1"`;
    - `Signal::from_number(0) == None` and `Signal::from_number(32) == None`.
  - `signal_contexts` (runtime): `let s = unix::signal!("SIGHUP");`, `static S: Signal = unix::signal!("SIGKILL");`, and use in a `match` guard (`x if x == unix::signal!("SIGINT")`).
  - `signal_libc` (`constlit/tests/signal_libc.rs`): a `const _: () = assert!(…)` per signal that libc defines on the family, comparing `__signals::<NAME> == Some(libc::SIG<NAME>)`, gated on the family `cfg`s; `SUPPORTED` is true. Skip `LOST`, `EMT` and `INFO` where libc lacks them; the table check in `signal_tables` covers those.
  - `signal_tables` (runtime unit test in `signal.rs`): for each family table, written out as plain arrays independent of `cfg`, every number 1-31 has exactly one canonical name (sparc 29 also has the alias `LOST`); `name()` for sparc 29 is `"SIGPWR"`; and the numbers match the signal table above row by row.
  - UI (`tests/ui`): `signal_number.rs` (`unix::signal!("15")`), `signal_lowercase.rs` (`"sigterm"`) and `signal_realtime.rs` (`"SIGRTMIN+3"`).
  - UI (`tests/ui_linux`): `signal_missing.rs`, `let _ = constlit::unix::signal!("SIGINFO");`. The snapshot must contain `SIGINFO does not exist on this target`.
- [ ] **Step 2: Run** `cargo test -p constlit-macros signal && cargo test -p constlit --all-features`. Expected: FAIL (items missing).
- [ ] **Step 3: Implement.** The family tables live once in `signal.rs` as `cfg`-free arrays of `(name, number)`. The `cfg`-gated `table` module selects one of them, and `Signal::name()` and `from_number` scan the selected array in a `const fn` loop.
- [ ] **Step 4: Run** the Global Constraints checks and `TRYBUILD=overwrite cargo test -p constlit --test ui`, then read the new `.stderr` files. Expected: PASS, no warnings.
- [ ] **Step 5: Cross-target checks.** Run:
  - `cargo check -p constlit --all-features --tests --target aarch64-apple-darwin` (bsd family, against libc's apple constants);
  - `cargo check -p constlit --all-features --tests --target aarch64-unknown-linux-gnu`;
  - `rustup target add sparc64-unknown-linux-gnu`, then the same check with that target (the sparc family). If rustup can't install it, ledger a ruling; `signal_tables` still pins the numbers.

  Expected: all finish with no errors, so every libc static assertion holds.

  Then make a scratch crate in the plan workspace (`.superpowers/sdd/<plan>/scratch-signal/`). It depends on `constlit` by path, and its `src/lib.rs` is `pub const S: constlit::unix::Signal = constlit::unix::signal!("SIGPWR");`.
  - `cargo check --target aarch64-apple-darwin`. Expected: error containing `SIGPWR does not exist on this target`.
  - `cargo check --target wasm32-unknown-unknown`. Expected: error containing `signal! supports Linux, Android, macOS, iOS and the BSDs`.
  - The host check passes.
- [ ] **Step 6: Commit** `git commit -m "unix::signal!: signal names as constlit::unix::Signal, numbered per target"`

---

### Task 4: `linux` module, `Capability`, `CapSet`, `cap!` and `caps!`

**Files:**
- Create: `constlit/src/linux/mod.rs`, `constlit/src/linux/cap.rs`, `constlit-macros/src/cap.rs`
- Modify:
  - `constlit/src/lib.rs`: `pub mod linux;`;
  - `constlit-macros/src/lib.rs`: `#[proc_macro] pub fn linux_cap` and `linux_caps`;
  - `constlit/Cargo.toml`: `[target.'cfg(target_os = "linux")'.dev-dependencies] caps = "0.5"`.
- Test:
  - unit tests in both new files;
  - `constlit/tests/cap.rs`;
  - UI: `constlit/tests/ui/cap_lowercase.rs`, `cap_all.rs` and `caps_duplicate.rs`.

**Interfaces:**
- Consumes `suggest::closest` (Task 3).
- Produces `constlit::linux` with:
  - module docs;
  - `pub use cap::{Capability, CapSet, CapSetIter};`;
  - the documented `pub use constlit_macros::linux_cap as cap;` and `linux_caps as caps;`.
- Produces `constlit::linux::Capability`:
  - private `u8`; derives `Clone`, `Copy`, `Debug`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Hash`;
  - `pub const fn from_number(n: u8) -> Option<Capability>` (`n <= 40`) and `#[doc(hidden)] pub const fn __new_unchecked(n: u8)`;
  - `const fn` methods `number() -> u8` and `name() -> &'static str`;
  - `Display` writes `name()`.
- Produces `constlit::linux::CapSet`:
  - private `u64`; derives as `Capability`;
  - `pub const EMPTY: CapSet`;
  - `pub const fn from_bits(bits: u64) -> Option<CapSet>` (`None` if any bit above 40 is set) and `#[doc(hidden)] pub const fn __from_bits_unchecked(bits: u64)`;
  - `const fn` methods `bits()`, `contains(Capability)`, `with(Capability)`, `without(Capability)`, `union(CapSet)`, `intersection(CapSet)`, `difference(CapSet)`, `len() -> u32` and `is_empty()`;
  - `iter() -> CapSetIter` (ascending), `impl IntoIterator for CapSet`, `FromIterator<Capability>`;
  - `Display` as decided.
- Produces `constlit_macros::cap::parse_cap(s: &str) -> Result<u8, String>` and `parse_caps(s: &str) -> Result<u64, String>`.
- `linux::cap!` expands to `::constlit::linux::Capability::__new_unchecked(12u8)`. `linux::caps!` expands to `::constlit::linux::CapSet::__from_bits_unchecked(…u64)`.

Error messages (exact):

| input | message |
|---|---|
| `""` (cap!) | `empty capability; write a name like "CAP_NET_ADMIN"` |
| `"cap_net_admin"`, `"net_admin"` | `capability names are upper case: "CAP_NET_ADMIN"` (upper-cased as written: `"NET_ADMIN"` for `"net_admin"`) |
| `"ALL"`, `"CAP_ALL"` | `"ALL" depends on the kernel; list the capabilities you need` |
| `"12"` | `write the capability name: "CAP_NET_ADMIN"` (0-40); otherwise `write the capability name, like "CAP_NET_ADMIN"` |
| `"CAP_NET_ADMN"` | `unknown capability "CAP_NET_ADMN"; did you mean "CAP_NET_ADMIN"?` (compared with the `CAP_` form) |
| `"CAP_FOO"` | `unknown capability "CAP_FOO"` |
| `""` (caps!) | `empty capability set; use CapSet::EMPTY` |
| `"CAP_KILL,"` | `remove the trailing ","` |
| `"CAP_KILL,,CAP_CHOWN"` | `empty name between commas` |
| `"CAP_KILL ,CAP_CHOWN"`, `"CAP_KILL,  CAP_CHOWN"` | `separate names with "," or ", "` |
| `"NET_ADMIN, CAP_NET_ADMIN"` | `CAP_NET_ADMIN is listed twice` |

In `caps!`, each name gets `cap!`'s messages.

- [ ] **Step 1: Write the failing tests.**
  - `cap_values` (macros):
    - `"CAP_NET_ADMIN"` and `"NET_ADMIN"` → `12`;
    - `"CAP_CHOWN"` → `0`;
    - `"CAP_CHECKPOINT_RESTORE"` → `40`.
  - `cap_errors` and `caps_errors` (macros): every row of the table.
  - `caps_values` (macros):
    - `"CAP_NET_ADMIN, CAP_SYS_TIME"` → `(1 << 12) | (1 << 25)`;
    - `"CHOWN,KILL"` → `0b100001`;
    - `"CAP_CHECKPOINT_RESTORE"` → `1 << 40`.
  - `cap_table_oracle` (runtime, in `constlit/tests/cap.rs`, `#[cfg(target_os = "linux")]`):
    - for each `c` in `caps::all()`, `Capability::from_number(c.index()).unwrap().name() == c.to_string()`;
    - the count is 41.

    Also a macros-side test with the same check against a 41-entry list the test copies from the plan's capability table. The `caps` crate is a dev-dep of the runtime crate only; the macros copy is pinned to the plan table.
  - `cap_accessors` (runtime):
    - `const C: Capability = linux::cap!("NET_BIND_SERVICE");` gives `number() == 10` and `to_string() == "CAP_NET_BIND_SERVICE"`;
    - `Capability::from_number(41) == None`.
  - `capset_ops` (runtime):
    - `const S: CapSet = linux::caps!("CAP_NET_ADMIN, CAP_SYS_TIME");` gives `S.len() == 2`, `S.contains(linux::cap!("CAP_NET_ADMIN"))` and `!S.contains(linux::cap!("CAP_KILL"))`;
    - `S.to_string() == "CAP_NET_ADMIN, CAP_SYS_TIME"`;
    - `S.iter().map(|c| c.number()).collect::<Vec<_>>() == [12, 25]`;
    - `CapSet::EMPTY.with(C).without(C).is_empty()`;
    - `S.union(CapSet::EMPTY) == S`, `S.intersection(CapSet::EMPTY) == CapSet::EMPTY`, `S.difference(S).is_empty()`;
    - `CapSet::from_bits(1 << 40).is_some()` and `CapSet::from_bits(1 << 41) == None`;
    - `S.iter().collect::<CapSet>() == S`;
    - `CapSet::EMPTY.to_string() == ""`.
  - UI: `cap_lowercase.rs` (`linux::cap!("cap_net_admin")`), `cap_all.rs` (`"ALL"`) and `caps_duplicate.rs` (`linux::caps!("NET_ADMIN, CAP_NET_ADMIN")`).
- [ ] **Step 2: Run** `cargo test -p constlit-macros cap && cargo test -p constlit --all-features`. Expected: FAIL (items missing).
- [ ] **Step 3: Implement** `parse_caps` by splitting on `,` and checking each separator: exactly `","` or `", "`. Run `parse_cap` on each name, and check for duplicates by number.
- [ ] **Step 4: Run** the Global Constraints checks, `TRYBUILD=overwrite cargo test -p constlit --test ui` (then read the new `.stderr` files), and `cargo check -p constlit --all-features --tests --target aarch64-apple-darwin` (the `caps` dev-dep is Linux-only, so the test must still compile elsewhere). Expected: PASS, no warnings.
- [ ] **Step 5: Commit** `git commit -m "linux::cap! and caps!: capabilities as constlit::linux::Capability and CapSet"`
