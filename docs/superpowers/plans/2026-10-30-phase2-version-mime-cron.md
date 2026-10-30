# constlit Phase 2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `version!`, `mime!` and `cron!` to constlit, with their own types (`Version`, `MediaType`, `CronSpec`), `CronSpec::matches`, `next_after` for `jiff`/`chrono` date-times, and the `semver`/`mime` conversions.

**Architecture:** This follows phase 1.
- `constlit-macros` gets one pure parser module per macro, `parse_*(&str) -> Result<_, String>`, unit-tested with exact messages. Expansion goes through `input::expand`.
- `constlit` gets one module per own type. Fields are private, accessors are `const fn`, and a `#[doc(hidden)] pub const fn __new_unchecked(..)` builds values for the macros.
- Scheduling arithmetic lives in a private `calendar` module that phase 3's `Date` will reuse.

**Tech Stack:**
- Same as phase 1: Rust 1.88+, edition 2024, `proc-macro2`, `quote`, `litrs`, `trybuild`.
- New optional deps: `semver` 1, `mime` 0.3, `jiff` 0.2, `chrono` 0.4.
- New dev-deps: those four, plus `croner` 4 as the cron oracle.

**Spec:** `docs/superpowers/specs/2026-10-30-constlit-design.md`. Relevant sections: macros 6-8, Own types, Errors, Testing, and Conversions in version one.

**Starting point:** `main` at `af175ff` (phase 1 merged).
- `constlit-macros/src/input.rs` provides `expand`, `string_literal`, `check_spaces`, `decimal`/`Decimal`/`Scale` and `unexpected_after`.
- `net.rs` shows the parser/emitter pattern.
- The UI tests live in `constlit/tests/ui/*.rs` with `.stderr` snapshots, and `constlit/tests/ui.rs` runs them all.

## Global Constraints

- Every phase 1 constraint still holds:
  - stable Rust ≥ 1.88, edition 2024;
  - `constlit` is `#![no_std]`, with no allocation outside opt-in features;
  - no `unsafe`;
  - every error is a `compile_error!` spanned on the literal, naming the problem and the nearest valid literal where one exists;
  - no silent ambiguity;
  - expansions work in `const` items.
- Literal whitespace: leading or trailing whitespace is always an error, with the message `remove the spaces around the literal`. Separators inside the literal follow each macro's own rule below.
- New Cargo features in `constlit`, all off by default:
  - `semver = ["dep:semver"]`;
  - `mime = ["dep:mime"]` (needs `std`; say so in the feature's doc line);
  - `jiff = ["dep:jiff"]`;
  - `chrono = ["dep:chrono"]`.

  Each optional dep uses `default-features = false` plus the minimum features it needs:
  - `semver`: none;
  - `jiff`: `alloc` (`Zoned` needs it);
  - `chrono`: none.

  Phase 3 reuses the `jiff` and `chrono` feature names.
- Dev-dependencies:
  - `semver = "1"`;
  - `mime = "0.3"`;
  - `croner = "4"`;
  - `jiff = { version = "0.2", features = ["tzdb-bundle-always"] }`, so time-zone tests don't depend on the host's zoneinfo;
  - `chrono = "0.4"` and `chrono-tz = "0.10"`, for the chrono DST test.
- MSRV check: `cargo +1.88 build -p constlit --features semver,mime,jiff,chrono`. The dev-deps may need newer Rust, so tests run on stable.

## Review Focus

1. **`version!("v1.2.3")`.** Writing the leading `v` from git tags is the most common mistake. It must be an error suggesting `"1.2.3"`, not a generic message. Pinned by `version_errors` (Task 1).
2. **Huge numeric pre-release identifiers** (`"1.0.0-alpha.99999999999999999999999"`) must parse. They must also order numerically, above `alpha.99`, without overflowing. SemVer doesn't bound them. Pinned by `precedence_big_numeric` (Task 1).
3. **Media-type wildcards** (`"text/*"`, `"*/*"`) must be an error explaining that they are media ranges for `Accept` headers, not media types. Pinned by `mime_errors` (Task 2).
4. **Cron with a seconds field or Quartz syntax** (six fields, `?`, `L`, `W`, `#`) must be an error naming the dialect, not "invalid character". Pinned by `cron_dialect_errors` (Task 3).
5. **`next_after` at or inside a matching minute.** At `10:00:30` with `"0 10 * * *"` the answer is the next day's 10:00, not now. At exactly `10:00:00` it is also the next day. Results are always strictly after `t`, with seconds and nanoseconds zero. Pinned by `next_after_strictly_after` (Task 4).

---

### Task 1: `Version` and `version!`

**Files:**
- Create: `constlit/src/version.rs`, `constlit-macros/src/version.rs`
- Modify:
  - `constlit/Cargo.toml` (feature `semver`, optional dep and dev-dep);
  - `constlit-macros/Cargo.toml` (dev-dep `semver = "1"` for the oracle);
  - `constlit/src/lib.rs` (`mod version; pub use version::Version;` and a documented `pub use constlit_macros::version;`);
  - `constlit-macros/src/lib.rs` (`#[proc_macro] pub fn version`).
- Test:
  - unit tests in both `version.rs` files;
  - `constlit/tests/version.rs`;
  - `constlit/tests/ui/version_leading_v.rs` and `version_missing_patch.rs`.

**Interfaces:**
- Consumes: `input::expand` (phase 1).
- Produces, in `constlit::Version`:
  - Private fields: `major: u64`, `minor: u64`, `patch: u64`, `pre: &'static str`, `build: &'static str`. `pre` and `build` are stored without their `-` or `+`, and are `""` when absent.
  - `pub const fn new(major: u64, minor: u64, patch: u64) -> Self`. There is no pre-release or build, so it can't fail.
  - `#[doc(hidden)] pub const fn __new_unchecked(major, minor, patch, pre: &'static str, build: &'static str) -> Self`.
  - `const fn` accessors `major()`, `minor()`, `patch()`, `pre() -> &'static str` and `build() -> &'static str`.
  - Manual `PartialEq`, `Eq`, `PartialOrd`, `Ord` and `Hash`, all ignoring `build`, so that `Eq` and `Hash` agree with SemVer precedence.
  - `Debug` (derived) and `Display` (canonical `1.2.3-pre+build`).
  - With `semver`: `pub fn to_semver(&self) -> semver::Version`. It uses `Prerelease::new` and `BuildMetadata::new` with `.expect("validated by version!")`.
- Produces, in macros: `pub fn parse_version(s: &str) -> Result<(u64, u64, u64, String, String), String>`. `version!` expands to `::constlit::Version::__new_unchecked(M, m, p, "pre", "build")`.

**Precedence (`Ord`):**
- Compare `(major, minor, patch)` first.
- Then an empty `pre` ranks above any non-empty `pre`.
- Otherwise compare the dot-separated identifiers pairwise:
  - numeric with numeric: compare by length, then bytewise, which is valid because there are no leading zeros and works for any size;
  - numeric ranks below alphanumeric;
  - alphanumeric with alphanumeric: compare bytewise (ASCII);
  - when every compared identifier is equal, the shorter list ranks lower.

**Grammar:**
- SemVer 2.0.0 exactly.
- Identifiers are `[0-9A-Za-z-]+`.
- Numeric pre-release identifiers can't have leading zeros. Build identifiers may.

Error messages (exact):

| input | message |
|---|---|
| `""` | `empty version; write MAJOR.MINOR.PATCH, like "1.2.3"` |
| `"v1.2.3"`, `"V1.2.3"` | `remove the leading "v"; write "1.2.3"` |
| `"1"` | `missing minor and patch versions; write e.g. "1.0.0"` |
| `"1.2"` | `missing patch version; write e.g. "1.2.0"` |
| `"1.2.3.4"` | `too many components; SemVer has MAJOR.MINOR.PATCH, like "1.2.3"` |
| `"1.x.3"` | ``minor version `x` is not a number`` (also `major`/`patch`; an empty component is ``minor version `` is not a number``) |
| `"01.2.3"` | `major version can't have leading zeros; write "1.2.3"` (suggestion is the input with that component's zeros stripped) |
| `"99999999999999999999.0.0"` | `major version is too large for u64` |
| `"1.2.3-"` | `empty pre-release; remove the "-" or write e.g. "1.2.3-beta.1"` |
| `"1.2.3+"` | `empty build metadata; remove the "+" or write e.g. "1.2.3+build.5"` |
| `"1.2.3-beta..1"` | `empty identifier in pre-release "beta..1"` (same for build: `in build metadata "…"`) |
| `"1.2.3-01"` | `numeric pre-release identifier "01" can't have leading zeros; write "1"` |
| `"1.2.3-beta_1"` | ``invalid character `_` in pre-release; use letters, digits, hyphens and dots`` (same for build) |

- [ ] **Step 1: Write the failing tests.**
  - `version_values`:
    - `"1.2.3"` → `(1, 2, 3, "", "")`;
    - `"1.2.3-beta.1+build.5"` → `(1, 2, 3, "beta.1", "build.5")`;
    - `"0.0.0"`;
    - `"1.0.0+001"`: leading zero in build is OK;
    - `"1.0.0-x-y-z.--"`;
    - `"18446744073709551615.0.0"`.
  - `version_errors`: every row of the table.
  - `precedence` (runtime): the SemVer 2.0.0 example chain is strictly increasing under `Ord`, built through `Version::__new_unchecked`. The chain is `1.0.0-alpha < 1.0.0-alpha.1 < 1.0.0-alpha.beta < 1.0.0-beta < 1.0.0-beta.2 < 1.0.0-beta.11 < 1.0.0-rc.1 < 1.0.0`. Also `1.0.0 < 2.0.0 < 2.1.0 < 2.1.1`, and `1.0.0+a == 1.0.0+b`, with equal hashes.
  - `precedence_big_numeric`: `1.0.0-alpha.99 < 1.0.0-alpha.99999999999999999999999 < 1.0.0-alpha.a`.
  - `display` (runtime): round-trips `"1.2.3"`, `"1.2.3-rc.1"` and `"1.2.3-rc.1+b.7"`.
  - `version_in_const` (`constlit/tests/version.rs`): `const V: Version = version!("1.2.3-rc.1");` with `V.pre() == "rc.1"` and `V > version!("1.2.3-beta.9")`.
  - `version_oracle`, in the macros crate's unit tests. A proc-macro crate can't export `parse_version`, so `semver` is a dev-dependency of `constlit-macros`. Check 20,000 random strings over the alphabet `0123456789.-+abv` (length 0-14), plus 20,000 well-formed random versions: `parse_version` accepts exactly what `semver::Version::parse` accepts, with the same fields.
  - `precedence_oracle` (`constlit/tests/version.rs`): for 20,000 random pairs of well-formed versions, `Ord` on values built with `Version::__new_unchecked` from `semver`'s parsed fields agrees with `semver::Version::cmp_precedence`.
  - `to_semver` (`#[cfg(feature = "semver")]`): `version!("1.2.3-rc.1+b").to_semver() == semver::Version::parse("1.2.3-rc.1+b").unwrap()`.
  - UI files: `version_leading_v.rs` (`version!("v1.2.3")`) and `version_missing_patch.rs` (`version!("1.2")`).
- [ ] **Step 2: Run** `cargo test -p constlit-macros version && cargo test -p constlit`. Expected: FAIL (items missing).
- [ ] **Step 3: Implement.** Precedence lives in one private `fn cmp_pre(a: &str, b: &str) -> Ordering`.
- [ ] **Step 4: Run** `cargo test --workspace && cargo test -p constlit --features semver`, then `TRYBUILD=overwrite cargo test -p constlit --test ui` and read each new `.stderr`: the caret must be on the literal, with the exact message. Expected: PASS.
- [ ] **Step 5: Commit** `git commit -m "version!: SemVer literals as constlit::Version"`

---

### Task 2: `MediaType` and `mime!`

**Files:**
- Create: `constlit/src/media_type.rs`, `constlit-macros/src/media_type.rs`
- Modify:
  - `constlit/Cargo.toml` (feature `mime`; optional dep `mime = { version = "0.3", optional = true }`; dev-dep);
  - `constlit-macros/Cargo.toml` (dev-dep `mime = "0.3"` for the oracle);
  - both `lib.rs` files.
- Test:
  - unit tests in both files;
  - `constlit/tests/media_type.rs`;
  - `constlit/tests/ui/mime_wildcard.rs` and `mime_missing_slash.rs`.

**Interfaces:**
- Consumes: `input::expand`.
- Produces, in `constlit::MediaType`:
  - Private fields: `essence: &'static str` (lowercased `type/subtype`), `slash: usize` and `params: &'static [(&'static str, &'static str)]`. Param names are lowercase. Values are stored unquoted and unescaped, and a `charset` value is lowercased.
  - `#[doc(hidden)] pub const fn __new_unchecked(essence, slash, params) -> Self`.
  - `const fn` accessors `essence()`, `type_()`, `subtype()` and `params()`. Use `str::split_at` in const; if it isn't const on 1.88, make `type_`/`subtype` non-const and ledger the ruling.
  - `pub fn param(&self, name: &str) -> Option<&'static str>`, matching the name ASCII case-insensitively.
  - `Display`: the essence, then `; name=value` for each parameter. A value goes out as-is when it is a token; otherwise it is quoted, with `"` and `\` escaped.
  - Derives `Clone`, `Copy`, `Debug`, `PartialEq`, `Eq` and `Hash`, structurally.
  - With `mime`: `pub fn to_mime(&self) -> mime::Mime`, which parses `self.to_string()` with `.expect("validated by mime!")`. The module uses `extern crate alloc` under the feature.
- Produces, in macros: `pub fn parse_media_type(s: &str) -> Result<(String, usize, Vec<(String, String)>), String>`. `mime!` expands to `::constlit::MediaType::__new_unchecked("text/html", 4, &[("charset", "utf-8")])`.

**Grammar (decided):**
- `type "/" subtype *( OWS ";" OWS name "=" value )`.
- `type` and `subtype` are RFC 6838 restricted-names: 1-127 chars, the first alphanumeric, the rest alphanumeric or ``!#$&-^_.+``.
- `name` is an RFC 9110 token: ``!#$%&'*+-.^_`|~``, digits and letters.
- `value` is a token or a quoted-string (`"…"` with `\`-escapes). An empty quoted `""` is allowed.
- OWS (spaces or tabs) is allowed only around `;`.

Error messages (exact):

| input | message |
|---|---|
| `""` | `empty media type; write type/subtype, like "application/json"` |
| `"json"` | ``missing `/`; write type/subtype, like "application/json"`` |
| `"text/"` | `empty subtype; write e.g. "text/plain"` |
| `"/plain"` | `empty type; write e.g. "text/plain"` |
| `"text/*"`, `"*/*"` | `` `*` wildcards are media ranges for Accept headers, not media types `` |
| `"te xt/plain"` | ``invalid character ` ` in type `te xt` `` (same pattern for `subtype` and `parameter name`) |
| `"text/plain;"` | ``remove the trailing `;` `` |
| `"text/plain; charset"` | ``parameter `charset` needs a value, like charset=utf-8`` (the example uses the name written) |
| `"text/plain; charset="` | ``parameter `charset` has an empty value`` |
| `"text/plain; charset = utf-8"` | ``no spaces around `=` in parameter `charset` `` |
| `"text/plain; a=1; A=2"` | ``parameter `a` appears twice`` |
| `"text/plain; a=\"x"` | ``unterminated quoted value in parameter `a` `` |

- [ ] **Step 1: Write the failing tests.**
  - `mime_values`:
    - `"application/json"` → `("application/json", 11, [])`;
    - `"Text/HTML; Charset=UTF-8"` → `("text/html", 4, [("charset", "utf-8")])`;
    - `"multipart/form-data; boundary=\"a b\\\"c\""` → `boundary` = `a b"c`;
    - `"application/vnd.api+json"`;
    - `"text/plain;format=flowed"` (no OWS) works.
  - `mime_errors`: every row.
  - `display` (runtime):
    - `__new_unchecked("multipart/form-data", 9, &[("boundary", "a b\"c")])` shows `multipart/form-data; boundary="a b\"c"`;
    - token values aren't quoted.
  - `accessors`: `type_() == "text"`, `subtype() == "html"`, `param("CHARSET") == Some("utf-8")`, `param("x") == None`.
  - `mime_in_const` (`constlit/tests/media_type.rs`): `const JSON: MediaType = mime!("application/json");`.
  - `mime_oracle` (macros unit tests, `mime` dev-dep): for 5,000 random well-formed media types (random restricted-names, 0-3 params with token or quoted values), the `mime` crate parses our normalized output. Its `essence_str()` equals our essence, and its params equal ours by name and value, with `charset` compared ASCII-case-insensitively.
  - `to_mime` (`#[cfg(feature = "mime")]`): `mime!("text/html; charset=utf-8").to_mime() == mime::TEXT_HTML_UTF_8`.
  - UI files: `mime_wildcard.rs` and `mime_missing_slash.rs`.
- [ ] **Step 2: Run** `cargo test -p constlit-macros media_type && cargo test -p constlit`. Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `cargo test --workspace && cargo test -p constlit --features mime`, then the UI snapshot review as in Task 1. Expected: PASS.
- [ ] **Step 5: Commit** `git commit -m "mime!: media types as constlit::MediaType"`

---

### Task 3: `CronSpec`, `cron!` and `matches`

**Files:**
- Create: `constlit/src/cron.rs`, `constlit-macros/src/cron.rs`
- Modify: both `lib.rs` files
- Test:
  - unit tests in both files;
  - `constlit/tests/cron.rs`;
  - `constlit/tests/ui/cron_seconds.rs`, `cron_reboot.rs` and `cron_out_of_range.rs`.

**Interfaces:**
- Consumes: `input::expand`.
- Produces, in `constlit::CronSpec`:
  - Private fields:
    - `source: &'static str`;
    - `minutes: u64` (bit m for minute m);
    - `hours: u32`;
    - `days: u32` (bit d, d in 1..=31);
    - `months: u16` (bit m, m in 1..=12);
    - `weekdays: u8` (bit w, w in 0..=6, Sunday = 0; a `7` in the literal sets bit 0);
    - `dom_star: bool`, `dow_star: bool` (the field's text starts with `*`).
  - `#[doc(hidden)] pub const fn __new_unchecked(source, minutes, hours, days, months, weekdays, dom_star, dow_star) -> Self`.
  - `const fn` accessors: `as_str()`, `minutes()`, `hours()`, `days_of_month()`, `months()` and `days_of_week()`.
  - `pub const fn matches(&self, minute: u8, hour: u8, day: u8, month: u8, weekday: u8) -> bool`. Out-of-range arguments give `false`; `weekday` is 0-6 with Sunday = 0.
  - Derives `Clone`, `Copy`, `Debug`, `PartialEq`, `Eq` and `Hash`, structurally, so `@daily` != `0 0 * * *` (document this).
  - `Display` writes `source`.
- Produces, in macros: `pub struct Cron { pub minutes: u64, pub hours: u32, pub days: u32, pub months: u16, pub weekdays: u8, pub dom_star: bool, pub dow_star: bool }` and `pub fn parse_cron(s: &str) -> Result<Cron, String>`. `cron!` expands to `::constlit::CronSpec::__new_unchecked("<literal>", ..)`.

**Day rule (spec, Vixie):** minute, hour and month must each match. Then:
- if `dom_star || dow_star`: day matches when `day_bit && weekday_bit`;
- otherwise: day matches when `day_bit || weekday_bit`.

**Grammar (decided):**
- Fields are separated by one or more spaces or tabs. Leading or trailing whitespace is an error, using the phase 1 message.
- A field is a `,`-list of items. The item forms are `*`, `*/n`, `a`, `a-b`, `a-b/n` and `a/n`. `a/n` means `a-max/n`.
- Names (`JAN`-`DEC` in the month field, `SUN`-`SAT` in the day-of-week field) are case-insensitive and work as values and range ends.
- Day of week accepts 0-7. `5-7` is Friday through Sunday.
- Shortcuts are lowercase only:
  - `@yearly` and `@annually` = `0 0 1 1 *`;
  - `@monthly` = `0 0 1 * *`;
  - `@weekly` = `0 0 * * 0`;
  - `@daily` and `@midnight` = `0 0 * * *`;
  - `@hourly` = `0 * * * *`.

Field names and ranges for messages:

| field | range |
|---|---|
| minute | 0-59 |
| hour | 0-23 |
| day of month | 1-31 |
| month | 1-12 |
| day of week | 0-7 |

Error messages (exact):

| input | message |
|---|---|
| `""` | `empty cron expression; write 5 fields like "*/5 * * * *" or a shortcut like "@daily"` |
| `"* * * *"` | `expected 5 fields (minute hour day-of-month month day-of-week), found 4` |
| `"0 * * * * *"` | `expected 5 fields (minute hour day-of-month month day-of-week), found 6; seconds fields aren't supported` |
| `"60 * * * *"` | `minute 60 is out of range (0-59)` (all fields follow this pattern) |
| `"5-1 * * * *"` | `` range `5-1` goes backwards; cron ranges don't wrap around `` |
| `"*/0 * * * *"` | `` step `/0` must be at least 1 `` |
| `"0 0 ? * *"` | `` `?` is Quartz syntax; use `*` `` |
| `"0 0 L * *"`, `"0 0 15W * *"`, `"0 0 * * 5#3"` | `` `L`, `W` and `#` are not standard cron and aren't supported `` |
| `"0 0 * FOO *"` | ``unknown month name `FOO`; use JAN-DEC`` (`unknown day name `…`; use SUN-SAT` for day of week; a name in another field gives ``names are only allowed in the month and day-of-week fields``) |
| `"@reboot"` | `@reboot runs at startup, not on a schedule, so it has no CronSpec` |
| `"@every 5m"`, `"@Daily"` | `` unknown shortcut `@every`; use @yearly, @annually, @monthly, @weekly, @daily, @midnight or @hourly `` (the name shown is the word written) |
| `"1,,2 * * * *"` | `empty item in minute field` |
| other characters | ``invalid character `x` in minute field`` |

- [ ] **Step 1: Write the failing tests.**
  - `cron_values`:
    - `"*/15 * * * *"` gives minutes bits {0, 15, 30, 45} and every hour;
    - `"0 9-17 * * MON-FRI"` gives hours 9..=17 and weekdays {1..=5}, with `dom_star` true and `dow_star` false;
    - `"0 0 1,15 * *"`;
    - `"5/20 * * * *"` gives {5, 25, 45};
    - `"0 0 * * 5-7"` gives weekdays {5, 6, 0};
    - `"0 0 * * 7"` gives {0};
    - `"0 0 * jan,Jul *"`;
    - every shortcut equals its expansion's masks and flags.
  - `cron_errors` and `cron_dialect_errors`: every row of the table.
  - `matches_vixie` (runtime, `constlit/src/cron.rs`):
    - `0 0 */2 * MON` (`dom_star` true, so AND): day 3 Monday matches, day 4 Monday doesn't, day 3 Tuesday doesn't;
    - `0 0 1 * MON` (both restricted, so OR): day 1 Wednesday matches, day 8 Monday matches, day 9 Tuesday doesn't;
    - out-of-range arguments give `false`.
  - `cron_in_const` (`constlit/tests/cron.rs`): `const EVERY_5: CronSpec = cron!("*/5 * * * *");` with `as_str() == "*/5 * * * *"` and `to_string()` the same.
  - UI files: `cron_seconds.rs`, `cron_reboot.rs` and `cron_out_of_range.rs`.
- [ ] **Step 2: Run** `cargo test -p constlit-macros cron && cargo test -p constlit`. Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `cargo test --workspace`, then the UI snapshot review. Expected: PASS.
- [ ] **Step 5: Commit** `git commit -m "cron!: cron expressions as constlit::CronSpec with matches()"`

---

### Task 4: `next_after` for jiff and chrono

**Files:**
- Create: `constlit/src/calendar.rs` (private civil date arithmetic) and `constlit/src/cron_time.rs` (the `CronTime` trait and its feature-gated impls)
- Modify: `constlit/src/cron.rs` (`next_civil_after`, `next_after`)
- Modify: `constlit/Cargo.toml` (features `jiff` and `chrono`, optional deps, and dev-deps `croner`, `jiff` with `tzdb-bundle-always`, `chrono` and `chrono-tz`), `constlit/src/lib.rs`
- Test: unit tests in `calendar.rs` and `cron.rs`, `constlit/tests/cron_next.rs`

**Interfaces:**
- Consumes: `CronSpec` and `matches` (Task 3).
- Produces in `calendar.rs` (private, reused by phase 3):
  - `pub(crate) const fn is_leap(year: i32) -> bool`
  - `pub(crate) const fn days_in_month(year: i32, month: u8) -> u8`
  - `pub(crate) const fn weekday(year: i32, month: u8, day: u8) -> u8` (Sunday = 0; Sakamoto's method)
  - `pub(crate) struct Minute { year: i32, month: u8, day: u8, hour: u8, minute: u8 }` with `fn next(self) -> Minute`
- Produces in `CronSpec`:
  - `pub(crate) fn next_civil_after(&self, t: Minute) -> Option<Minute>`: the first matching `Minute` strictly after `t`, searching at most 400 years, which is one full Gregorian cycle, so finding nothing means it never matches.
  - Skip-ahead loop, so an impossible schedule like `0 0 30 2 *` costs at most about 12k iterations:
    - wrong month: jump to the 1st of the next month at 00:00;
    - wrong day under the day rule: jump to the next day at 00:00;
    - wrong hour: jump to the next hour at :00;
    - wrong minute: go to the next minute.
- Produces publicly:
  - `pub trait CronTime: Sized + sealed::Sealed`, with no public methods, documented as "a date-time type `next_after` works with".
  - Impls behind features:
    - `jiff`: `jiff::civil::DateTime` and `jiff::Zoned`;
    - `chrono`: `chrono::NaiveDateTime` and `chrono::DateTime<Tz: chrono::TimeZone>`.
  - `pub fn next_after<T: CronTime>(&self, t: &T) -> Option<T>`, compiled only when `jiff` or `chrono` is enabled.

**Zoned rule (spec):**
- Work on local wall-clock minutes in `t`'s zone.
- For each civil candidate from `next_civil_after`, resolve it in the zone:
  - **Gap:** step forward one local minute at a time until a minute exists, and use that instant. This is the first instant after the gap.
  - **Overlap:** take the earlier instant.
- If the resolved instant is not strictly after `t`, continue from that candidate. This makes jobs in a repeated hour run once.
- **jiff:** use `TimeZone::to_ambiguous_zoned(dt)`, with `is_gap()`/`is_fold()` and `earlier()`.
- **chrono:** use `tz.from_local_datetime(&naive)`, which returns `LocalResult::{None, Single, Ambiguous(earliest, _)}`.
- Results have seconds and nanoseconds set to zero. Civil `t` values with seconds are truncated to the minute before searching.

- [ ] **Step 1: Write the failing tests.**
  - `calendar_basics`:
    - `is_leap`: 2000 true, 1900 false, 2024 true;
    - `days_in_month(2026, 2) == 28`;
    - `weekday(2026, 10, 30) == 5` (Friday);
    - `weekday(2000, 1, 1) == 6`.
  - `next_civil`:
    - `"*/15 * * * *"` after 10:07 gives 10:15;
    - `"0 0 29 2 *"` after 2026-03-01 gives 2028-02-29;
    - `"0 0 30 2 *"` gives `None`;
    - `"0 0 31 * *"` after 2026-04-15 gives 2026-05-31.
  - `next_after_strictly_after` (jiff civil): `"0 10 * * *"`:
    - at 2026-10-30T10:00:30 gives 2026-10-31T10:00:00;
    - at exactly 10:00:00 also gives the 31st;
    - at 09:59:59.999 gives the 30th at 10:00.
  - `dst_gap_jiff`, Europe/Stockholm, spring forward 2026-03-29, where 02:00 jumps to 03:00. Schedule `"30 2 * * *"`, starting at 2026-03-28T12:00 local:
    - gives 2026-03-29T03:00+02:00;
    - next after that gives 2026-03-30T02:30+02:00.
  - `dst_overlap_jiff`, autumn back 2026-10-25, where 03:00 returns to 02:00. Schedule `"30 2 * * *"`:
    - from 2026-10-24T12:00 gives 2026-10-25T02:30+02:00, the first occurrence;
    - next after that gives 2026-10-26T02:30+01:00, so it doesn't run again at 02:30+01:00.
  - `dst_chrono`: the same two scenarios with `chrono::DateTime<chrono_tz::Tz>` (`chrono_tz::Europe::Stockholm`) give the same instants.
  - `cron_oracle` (`constlit/tests/cron_next.rs`): 2,000 random expressions and starting UTC civil times are compared with `croner` 4 in its default (Vixie OR) mode, using `jiff::civil::DateTime`.
    - Random expressions have 1-3 items per field from the forms above.
    - Generate day-of-month and day-of-week fields without a `*/` prefix. Croner's star handling for `*/n` may differ; those cases stay pinned by `matches_vixie`.
    - If croner's API differs from expectations, adapt the call and ledger it. Don't drop the oracle.
- [ ] **Step 2: Run** `cargo test -p constlit --features jiff,chrono`. Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run:**
  - `cargo test --workspace --features constlit/jiff,constlit/chrono,constlit/semver,constlit/mime,constlit/ipnet`;
  - `cargo build -p constlit --target thumbv7m-none-eabi`, which needs no features;
  - `cargo +1.88 build -p constlit --features semver,mime,jiff,chrono`;
  - `cargo clippy --workspace --all-targets --all-features`.

  Expected: all PASS, with no clippy warnings.
- [ ] **Step 5: Commit** `git commit -m "CronSpec::next_after for jiff and chrono date-times"`
