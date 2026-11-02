# constlit Phase 3 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `date!`, `time!` and `ts!` to constlit, with their own types (`Date`, `Weekday`, `TimeOfDay`, `Timestamp`, `ZonedTimestamp`), named time zones resolved at compile time from jiff's bundled tz database, and the `jiff`/`chrono`/`std` conversions.

**Architecture:** This follows phases 1 and 2.
- `constlit-macros` gets a shared date/time grammar module (`datetime.rs`), one parser module for `ts!` (`ts.rs`), and a zone resolver (`zone.rs`) compiled only with the new `tz` feature. Expansion goes through `input::expand`.
- `constlit` gets one module per own type. Fields are private, accessors are `const fn`, and a `#[doc(hidden)] pub const fn __new_unchecked(..)` builds values for the macros.
- Calendar arithmetic stays in `constlit/src/calendar.rs` (from phase 2). The macros crate can't depend on `constlit` (that would be a cycle), so the civil-to-Unix-days computation it needs is written there a second time and both copies are checked against jiff.

**Tech Stack:**
- Same as phase 2: Rust 1.88+, edition 2024, `proc-macro2`, `quote`, `litrs`, `trybuild`, `jiff` 0.2 and `chrono` 0.4 behind features.
- New optional deps of `constlit-macros`, behind `tz`: `jiff` 0.2 and `jiff-tzdb` 0.1.
- New dev-deps of `constlit-macros`: `jiff`, `chrono` and `chrono-tz` (the zone oracle).

**Spec:** `docs/superpowers/specs/2026-10-30-constlit-design.md`. Relevant sections: Constraints (feature `tz`), Own types, macros 9-11, Errors, Testing, and Conversions in version one.

**Starting point:** `main` at `67af9e8` (phases 1 and 2 merged).
- `constlit-macros/src/input.rs` provides `expand` and `check_spaces`. `version.rs` and `cron.rs` show the parser pattern: a pure `parse_*(&str) -> Result<_, String>` with exact-message tables in unit tests.
- `constlit/src/calendar.rs` has `is_leap`, `days_in_month`, `weekday` (Sunday = 0) and the `Minute` type used by `CronSpec::next_after`. `constlit/src/lib.rs` marks the whole module `allow(dead_code)` without `jiff`/`chrono`.
- The UI tests live in `constlit/tests/ui/*.rs` with `.stderr` snapshots, and `constlit/tests/ui.rs` runs them all.

## Global Constraints

- Every earlier constraint still holds:
  - stable Rust ≥ 1.88, edition 2024;
  - `constlit` is `#![no_std]`, with no allocation outside opt-in features;
  - no `unsafe`;
  - every error is a `compile_error!` spanned on the literal, naming the problem and the nearest valid literal where one exists;
  - no silent ambiguity;
  - expansions work in `const` items;
  - leading or trailing whitespace is the error `remove the spaces around the literal`.
- No run-time parsing or time-zone lookup: `ts!` with a named zone expands to numbers and a `&'static str`.
- Years are `0001`-`9999` everywhere: in `date!`, in `ts!`, and in the local date of any `Timestamp`.
- Calendar: proleptic Gregorian. Weekday numbering is never a bare number in the API: `Date::weekday()` returns `Weekday`.
- Features:
  - `constlit`: `default = ["tz"]`; `tz = ["constlit-macros/tz"]`; `std = []` (adds `Timestamp::to_system_time`, with `extern crate std` under the feature). The existing `jiff` and `chrono` features gain the date/time conversions.
  - `constlit-macros`: no default features; `tz = ["dep:jiff", "dep:jiff-tzdb"]`, with `jiff = { version = "0.2", optional = true, default-features = false, features = ["std", "tzdb-bundle-always"] }` and `jiff-tzdb = { version = "0.1", optional = true }`. Only the bundled database is used, so builds don't depend on the host's zoneinfo.
  - The doc line of each feature in `constlit/Cargo.toml` says what it adds.
- Dev-deps of `constlit-macros`: `jiff = { version = "0.2", features = ["tzdb-bundle-always"] }`, `chrono = "0.4"`, `chrono-tz = "0.10"`.
- Checks at the end of every task:
  - `cargo test --workspace --all-features` and `cargo test --workspace`;
  - `cargo test -p constlit-macros` (without `tz`) and `cargo test -p constlit-macros --features tz`;
  - `cargo clippy --workspace --all-targets -- -D warnings`, once with no features, once with `--all-features`, and once each with `--features jiff`, `--features chrono` and `--no-default-features`. Pass each `--features` value as its own shell word;
  - `cargo build -p constlit --target thumbv7m-none-eabi` (default features, so `tz` is on);
  - `cargo +1.88 test --workspace --all-features`;
  - `cargo fmt --all --check`.

## Decisions this plan makes (my calls, within the spec)

- **`Weekday`:** an enum (`Sunday = 0` … `Saturday = 6`) with chrono's method names, `number_from_monday()` (1-7) and `num_days_from_sunday()` (0-6). A bare `u8` weekday would be the kind of silent ambiguity the spec rules out.
- **Equality of timestamps:** `Timestamp` and `ZonedTimestamp` compare and hash by instant only, like `chrono::DateTime`. So `ts!("2026-10-30T14:00Z") == ts!("2026-10-30T15:00+01:00")`. `Ord` is by instant too.
- **Strict spelling:** `T` and `Z` are uppercase, and there is exactly one `T` or one space between date and time. Two-digit years, US/European day orders, week dates, AM/PM and zone abbreviations are errors with a pointer to the strict form.
- **Offsets:** `±HH:MM`, `±HHMM` or `±HH:MM:SS`, with hours `00`-`23` and minutes and seconds `00`-`59`. `±HH` alone is an error suggesting `±HH:00`. `-00:00` is an error: RFC 3339 gives it the meaning "offset unknown".
- **Zone forms:** a zone name follows the time (or the offset) after one space. The RFC 9557 bracket form (`…+01:00[Europe/Stockholm]`) is an error suggesting the space form. `Z` directly before a zone name is an error.
- **Zone names:** jiff's lookup ignores ASCII case. `ts!` requires the canonical spelling and suggests it (`europe/stockholm` → `Europe/Stockholm`). Unknown names suggest the closest known name when it is within edit distance 3. `ZonedTimestamp::zone()` returns the name as written, so links like `Asia/Calcutta` are kept.
- **Display:**
  - `Date`: `2026-10-30`.
  - `TimeOfDay`: `HH:MM:SS`, plus `.` and the fraction with trailing zeros removed when the nanosecond isn't zero. So `time!("14:30")` shows `14:30:00`.
  - `Timestamp`: local date `T` local time, then `Z` for offset zero, `±HH:MM` otherwise, or `±HH:MM:SS` when the offset has seconds (historical local mean time).
  - `ZonedTimestamp`: the `Timestamp` form with the offset (never `Z`), a space, and the zone name. `ts!` accepts that output back.
- **Conversions:** method names follow phase 2 (`to_semver`, `to_mime`): `to_jiff`, `to_chrono`, `to_chrono_utc`, `to_jiff_zoned`, `to_system_time`. A conversion is a `const fn` when the target's constructor is `const` in the pinned version (`jiff::civil::Date::constant`, `jiff::civil::Time::constant`, `jiff::Timestamp::constant`, `chrono::NaiveDate::from_ymd_opt`, `chrono::NaiveTime::from_hms_nano_opt`). Otherwise it is a plain `fn`.
- **tzdb version in the docs:** a hidden proc macro `__tzdb_version!()` expands to a string literal: `jiff_tzdb::VERSION`, or `"unknown"` when that is `None`. The `ts!` docs include it with `#[doc = …]`, so the docs always name the database actually bundled. Without `tz`, the macro expands to `"none (the tz feature is off)"`.

## Review Focus

1. **Leap years across centuries.** `"1900-02-29"` is an error and `"2000-02-29"` is valid, and the same goes for `ts!`. Pinned by `date_values`/`date_errors` (Task 1).
2. **Before 1970 with a fraction.** `ts!("1969-12-31T23:59:59.5Z")` has `unix_seconds() == -1` and `nanosecond() == 500_000_000` (floor division), not `0` and a negative fraction. Pinned by `timestamp_before_epoch` (Task 3).
3. **Instants just outside the date range.** `ts!("0001-01-01T00:00+01:00")` lies in year 0 in UTC, and `ts!("9999-12-31T23:59-01:00")` in year 10000. Both must expand, keep their written local `date()`, and convert to jiff and chrono without panicking. Pinned by `timestamp_extremes` (Task 3).
4. **Historical offsets with seconds.** `ts!("1880-06-01 12:00 Europe/Paris")` resolves to local mean time `+00:09:21`, which `Display` shows as `+00:09:21` and `to_jiff_zoned` agrees with. Paris is a canonical zone; zones that the tz database has merged into another (Stockholm is now a link to Berlin) take that zone's history, so tests of historical offsets use canonical zones. Pinned by `zoned_lmt` (Task 4).
5. **Half-hour and negative daylight saving.** Australia/Lord_Howe moves its clocks by 30 minutes, and Europe/Dublin's winter time is the DST time in the tz rules. The fold message for `"2026-04-05 01:45 Australia/Lord_Howe"` names `+11:00` and `+10:30`, and Dublin resolves like any other zone. Pinned by `zone_errors` and `zone_oracle` (Task 4).

---

### Task 1: `Date`, `Weekday` and `date!`

**Files:**
- Create: `constlit/src/date.rs`, `constlit-macros/src/datetime.rs`
- Modify:
  - `constlit/src/calendar.rs` (add `day_of_year`);
  - `constlit/src/lib.rs`: `mod date; pub use date::{Date, Weekday};` and a documented `pub use constlit_macros::date;`. Drop the module-wide `allow(dead_code)` on `calendar`, which `Date` now uses, and put it on `Minute` and its impl instead;
  - `constlit-macros/src/lib.rs` (`#[proc_macro] pub fn date`).
- Test:
  - unit tests in both new files;
  - `constlit/tests/date.rs`;
  - `constlit/tests/ui/date_feb_30.rs` and `date_day_first.rs`.

**Interfaces:**
- Produces, in `constlit::Date`:
  - Private fields `year: i16`, `month: u8`, `day: u8`, in that order, so derived `Ord` is chronological. Derives `Clone`, `Copy`, `Debug`, `PartialEq`, `Eq`, `PartialOrd`, `Ord` and `Hash`.
  - `pub const fn new(year: i16, month: u8, day: u8) -> Option<Date>`: `None` outside year 1-9999 or for impossible dates.
  - `#[doc(hidden)] pub const fn __new_unchecked(year: i16, month: u8, day: u8) -> Date`.
  - `const fn` accessors `year()`, `month()` and `day()`, plus `weekday() -> Weekday` and `day_of_year() -> u16`.
  - `Display` as decided above.
  - With `jiff`: `to_jiff(&self) -> jiff::civil::Date`. With `chrono`: `to_chrono(&self) -> chrono::NaiveDate`.
- Produces `constlit::Weekday`: `#[repr(u8)]` enum, `Sunday = 0` … `Saturday = 6`, deriving `Clone`, `Copy`, `Debug`, `PartialEq`, `Eq` and `Hash`, with `const fn number_from_monday(self) -> u8` and `const fn num_days_from_sunday(self) -> u8`.
- Produces, in `constlit/src/calendar.rs`: `pub(crate) const fn day_of_year(year: i32, month: u8, day: u8) -> u16`.
- Produces, in `constlit-macros/src/datetime.rs`, for Tasks 2-4:
  - `pub fn parse_date(s: &str) -> Result<(i16, u8, u8), String>`;
  - `pub fn days_in_month(year: i16, month: u8) -> u8` and `pub const MONTH_NAMES: [&str; 12]` (`"January"` …).
- `date!` expands to `::constlit::Date::__new_unchecked(2026i16, 10u8, 30u8)`.

Error messages (exact). Suggestions show the corrected date; when the date is part of a `ts!` literal they still show only the date:

| input | message |
|---|---|
| `""` | `empty date; write YYYY-MM-DD, like "2026-10-30"` |
| `"2026-2-3"` | `write the month and day with two digits: "2026-02-03"` (any 1-digit month or day; the suggestion pads both) |
| `"26-10-30"` | `write the year with all four digits; "26" could be any century` |
| `"2026/10/30"`, `"2026.10.30"` | `use "-" between year, month and day: "2026-10-30"` |
| `"30-10-2026"`, `"10/30/2026"`, `"30.10.2026"` | `write the date year first, as YYYY-MM-DD (ISO 8601)` (the last part has four digits) |
| `"20261030"` | `add the dashes: "2026-10-30"` |
| `"2026-W44-5"`, `"2026-303"` | `write the date as YYYY-MM-DD; week and ordinal dates aren't supported` |
| `"2026-10"` | `missing day; write YYYY-MM-DD, like "2026-10-01"` |
| `"2026-1x-30"` | ``month `1x` is not a number`` (same pattern for `year` and `day`) |
| `"0000-01-01"`, `"12026-01-01"` | `year 0000 is out of range (0001-9999)` (the year as written) |
| `"2026-13-01"`, `"2026-00-10"` | `month 13 is out of range (01-12)` |
| `"2026-02-30"`, `"2026-10-00"` | `day 30 is out of range: February 2026 has 28 days` (the day as written, then the month's real length) |
| `"2026-10-30T14:00"`, `"2026-10-30 14:00"` | `date! takes only a date; use ts! for a date with a time and zone` |

- [ ] **Step 1: Write the failing tests.**
  - `date_values` (macros): `"2026-10-30"` → `(2026, 10, 30)`; `"0001-01-01"`; `"9999-12-31"`; `"2024-02-29"`; `"2000-02-29"`.
  - `date_errors` (macros): every row of the table, plus `"1900-02-29"` and `"2025-02-29"` → `day 29 is out of range: February 1900 has 28 days` (and 2025), and `"2026-04-31"` → `... April 2026 has 30 days`.
  - `date_oracle` (macros, `jiff` dev-dep): for all `month` in 0..=13 and `day` in 0..=32, and for 2,000 random years in 1..=9999 plus 1900, 2000, 2024 and 2100, `parse_date` of the zero-padded text succeeds exactly when `jiff::civil::Date::new(year, month, day)` does.
  - `calendar` (runtime unit test): `day_of_year(2026, 1, 1) == 1`, `(2026, 12, 31) == 365`, `(2024, 12, 31) == 366`, `(2024, 3, 1) == 61`.
  - `date_accessors` (runtime, `constlit/tests/date.rs`):
    - `const D: Date = date!("2026-10-30");` with year/month/day `2026`, `10`, `30`;
    - `D.weekday() == Weekday::Friday`, `Weekday::Friday.number_from_monday() == 5` and `num_days_from_sunday() == 5`, `Weekday::Sunday.number_from_monday() == 7`;
    - `D.day_of_year() == 303`;
    - `D.to_string() == "2026-10-30"`, and `date!("0001-01-01").to_string() == "0001-01-01"`;
    - `date!("2026-10-29") < D`, and `Date::new(2026, 2, 30) == None`, `Date::new(0, 1, 1) == None`, `Date::new(2026, 10, 30) == Some(D)`.
  - `date_jiff_oracle` (`#[cfg(feature = "jiff")]`): for 5,000 random valid dates built with `Date::new`, `to_jiff()` equals `jiff::civil::date(y, m, d)`, `weekday().number_from_monday()` equals jiff's `weekday().to_monday_one_offset()`, `day_of_year()` equals jiff's, and `to_string()` equals jiff's `to_string()`.
  - `date_chrono` (`#[cfg(feature = "chrono")]`): `date!("2024-02-29").to_chrono() == NaiveDate::from_ymd_opt(2024, 2, 29).unwrap()`.
  - UI files: `date_feb_30.rs` (`date!("2026-02-30")`) and `date_day_first.rs` (`date!("30.10.2026")`).
- [ ] **Step 2: Run** `cargo test -p constlit-macros date && cargo test -p constlit --all-features`. Expected: FAIL (items missing).
- [ ] **Step 3: Implement.** `parse_date` splits on the first non-digit separator to classify the shape (separator, part lengths) before checking ranges, so each malformed shape gets its own row's message.
- [ ] **Step 4: Run** the Global Constraints checks, then `TRYBUILD=overwrite cargo test -p constlit --test ui` and read each new `.stderr`: the caret must be on the literal, with the exact message. Expected: PASS, no warnings.
- [ ] **Step 5: Commit** `git commit -m "date!: ISO 8601 dates as constlit::Date"`

---

### Task 2: `TimeOfDay` and `time!`

**Files:**
- Create: `constlit/src/time_of_day.rs`
- Modify: `constlit-macros/src/datetime.rs` (add `parse_time`), both `lib.rs` files
- Test:
  - unit tests in both files;
  - `constlit/tests/time_of_day.rs`;
  - `constlit/tests/ui/time_24.rs` and `time_leap_second.rs`.

**Interfaces:**
- Produces, in `constlit::TimeOfDay`:
  - Private fields `hour: u8`, `minute: u8`, `second: u8`, `nanosecond: u32`, in that order. Same derives as `Date`.
  - `pub const fn new(hour: u8, minute: u8, second: u8, nanosecond: u32) -> Option<TimeOfDay>`.
  - `#[doc(hidden)] pub const fn __new_unchecked(hour, minute, second, nanosecond) -> TimeOfDay`.
  - `const fn` accessors `hour()`, `minute()`, `second()` and `nanosecond()`.
  - `Display` as decided above.
  - With `jiff`: `to_jiff(&self) -> jiff::civil::Time`. With `chrono`: `to_chrono(&self) -> chrono::NaiveTime`.
- Produces, in `constlit-macros/src/datetime.rs`: `pub fn parse_time(s: &str) -> Result<(u8, u8, u8, u32), String>`. Task 3 calls it on the time part of a `ts!` literal.
- `time!` expands to `::constlit::TimeOfDay::__new_unchecked(14u8, 30u8, 5u8, 250000000u32)`.

**Grammar:** `HH:MM`, `HH:MM:SS` or `HH:MM:SS.F` with 1-9 fraction digits, 24-hour clock.

Error messages (exact):

| input | message |
|---|---|
| `""` | `empty time; write HH:MM or HH:MM:SS, like "14:30"` |
| `"9:30"`, `"09:5"` | `write each part with two digits: "09:30"` (pads every part) |
| `"14"` | `missing minutes; write HH:MM, like "14:00"` |
| `"1430"`, `"143005"` | `add the colons: "14:30"` (and `"14:30:05"`) |
| `"14:30:05:10"` | `too many parts; write HH:MM:SS` |
| `"24:00"` | `hour 24 is out of range (00-23); midnight is "00:00"` |
| `"25:00"` | `hour 25 is out of range (00-23)` |
| `"14:60"` | `minute 60 is out of range (00-59)` |
| `"23:59:60"` | `second 60 is out of range (00-59); leap seconds aren't supported` |
| `"14:30:05,250"` | `use "." before the fraction: "14:30:05.250"` |
| `"14:30:05."` | `missing digits after "."` |
| `"14:30:05.1234567891"` | `at most 9 fractional digits (nanoseconds)` |
| `"14:30.5"` | `a fraction needs seconds: write HH:MM:SS.F` |
| `"2:30 PM"`, `"12:15am"` | `use the 24-hour clock: "14:30"` (and `"00:15"`) |
| `"14h30"` | `use the 24-hour clock HH:MM, like "14:30"` |
| `"14:30Z"`, `"14:30+01:00"` | `time! takes a time of day without a zone; remove "Z", or use ts!` (the zone part as written) |
| `"T14:30"` | `remove the leading "T": "14:30"` |
| `"14:3x"` | ``minute `3x` is not a number`` (same pattern for `hour` and `second`) |

- [ ] **Step 1: Write the failing tests.**
  - `time_values` (macros): `"14:30"` → `(14, 30, 0, 0)`; `"00:00:00"`; `"23:59:59.999999999"`; `"14:30:05.25"` → nanosecond `250_000_000`; `"14:30:05.000000001"` → `1`.
  - `time_errors` (macros): every row.
  - `time_accessors` (runtime, `constlit/tests/time_of_day.rs`):
    - `const T: TimeOfDay = time!("14:30:05.250");` with accessors `14`, `30`, `5`, `250_000_000`;
    - `T.to_string() == "14:30:05.25"`, `time!("14:30").to_string() == "14:30:00"`, `time!("00:00:00.000000001").to_string() == "00:00:00.000000001"`;
    - `time!("09:00") < T`; `TimeOfDay::new(24, 0, 0, 0) == None`; `TimeOfDay::new(0, 0, 0, 1_000_000_000) == None`.
  - `time_jiff_oracle` (`#[cfg(feature = "jiff")]`): for 5,000 random `TimeOfDay::new` values, `to_jiff()` equals `jiff::civil::Time::new(..)` and `to_string()` equals jiff's `to_string()`.
  - `time_chrono` (`#[cfg(feature = "chrono")]`): `time!("23:59:59.5").to_chrono() == NaiveTime::from_hms_milli_opt(23, 59, 59, 500).unwrap()`.
  - UI files: `time_24.rs` (`time!("24:00")`) and `time_leap_second.rs` (`time!("23:59:60")`).
- [ ] **Step 2: Run** `cargo test -p constlit-macros time && cargo test -p constlit --all-features`. Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the Global Constraints checks and the UI snapshot review as in Task 1. Expected: PASS.
- [ ] **Step 5: Commit** `git commit -m "time!: times of day as constlit::TimeOfDay"`

---

### Task 3: `Timestamp` and `ts!` with `Z` and offsets

**Files:**
- Create: `constlit/src/timestamp.rs`, `constlit-macros/src/ts.rs`
- Modify:
  - `constlit/src/calendar.rs` (add `civil_from_days`);
  - `constlit-macros/src/datetime.rs` (add `days_from_civil`);
  - `constlit/Cargo.toml` (feature `std`), both `lib.rs` files.
- Test:
  - unit tests in `ts.rs`, `datetime.rs` and `calendar.rs`;
  - `constlit/tests/timestamp.rs`;
  - `constlit/tests/ui/ts_missing_zone.rs` and `ts_short_offset.rs`.

**Interfaces:**
- Consumes: `parse_date`, `parse_time` (Tasks 1-2), `Date`, `TimeOfDay`.
- Produces, in `constlit::Timestamp`:
  - Private fields `unix_seconds: i64`, `nanosecond: u32` (always below 1e9; the instant is `unix_seconds + nanosecond / 1e9`, so floor division before 1970), `offset_seconds: i32`.
  - `pub const fn new(unix_seconds: i64, nanosecond: u32, offset_seconds: i32) -> Option<Timestamp>`: `None` when `nanosecond ≥ 1e9`, when `|offset_seconds| ≥ 86_400`, or when the local date is outside years 1-9999.
  - `#[doc(hidden)] pub const fn __new_unchecked(unix_seconds, nanosecond, offset_seconds) -> Timestamp`.
  - `const fn` accessors `unix_seconds()`, `nanosecond()` and `offset_seconds()`; `const fn date(&self) -> Date` and `const fn time(&self) -> TimeOfDay`, both local to the offset.
  - Manual `PartialEq`, `Eq`, `PartialOrd`, `Ord` and `Hash` over `(unix_seconds, nanosecond)`; derived `Clone`, `Copy`, `Debug`. `Display` as decided above.
  - With `jiff`: `const fn to_jiff(&self) -> jiff::Timestamp`. With `chrono`: `to_chrono(&self) -> chrono::DateTime<chrono::FixedOffset>` (in its own offset) and `to_chrono_utc(&self) -> chrono::DateTime<chrono::Utc>`. With `std`: `to_system_time(&self) -> std::time::SystemTime`, working before 1970 too.
- Produces, in `constlit/src/calendar.rs`: `pub(crate) const fn civil_from_days(days: i64) -> (i32, u8, u8)` (days since 1970-01-01, Howard Hinnant's algorithm).
- Produces, in `constlit-macros/src/datetime.rs`: `pub fn days_from_civil(year: i16, month: u8, day: u8) -> i64` (the inverse, same algorithm).
- Produces, in `constlit-macros/src/ts.rs`:
  - `pub enum Ts { Fixed { secs: i64, nanos: u32, offset: i32 }, Zoned { secs: i64, nanos: u32, offset: i32, zone: String } }`;
  - `pub fn parse_ts(s: &str) -> Result<Ts, String>`. In this task a zone name gives the error `named time zones need constlit's "tz" feature; enable it, or write an offset like "+01:00"`. Task 4 replaces that path when `tz` is on.
- `ts!` expands to `::constlit::Timestamp::__new_unchecked(secs, nanos, offset)`.

**Grammar:** `DATE ("T" | " ") TIME ( "Z" | OFFSET ) [ " " ZONE ]`, or `DATE ("T" | " ") TIME " " ZONE`. Split the literal at the date/time separator, then take the time up to the first `Z`, `z`, `+`, `-`, `[` or space. The date and time parts produce their own Task 1-2 errors.

Error messages (exact):

| input | message |
|---|---|
| `""` | `empty timestamp; write a date, a time and a zone, like "2026-10-30T14:00Z"` |
| `"2026-10-30"` | `missing time; write a date, a time and a zone, like "2026-10-30T14:00Z"` |
| `"2026-10-30T14:00"`, `"2026-10-30 14:00"` | `missing time zone; add "Z" for UTC, an offset like "+01:00", or a zone name like "Europe/Stockholm"` |
| `"2026-10-30t14:00Z"` | `write "T" in uppercase: "2026-10-30T14:00Z"` |
| `"2026-10-30T14:00z"` | `write "Z" in uppercase: "2026-10-30T14:00Z"` |
| `"2026-10-30  14:00Z"`, `"2026-10-30_14:00Z"` | `separate the date and time with "T" or one space` |
| `"2026-10-30T14:00+02"`, `"…+2:00"` | `write the offset as ±HH:MM: "+02:00"` |
| `"2026-10-30T14:00+24:00"`, `"…+01:60"` | `offset +24:00 is out of range (-23:59:59 to +23:59:59)` |
| `"2026-10-30T14:00-00:00"` | `"-00:00" means an unknown offset (RFC 3339); write "Z" or "+00:00"` |
| `"2026-10-30T14:00 +01:00"` | `remove the space before the offset: "14:00+01:00"` |
| `"2026-10-30T14:00Z Europe/Stockholm"` | `write an offset like "+01:00" before a zone name, or remove "Z"` |
| `"2026-10-30T14:00+01:00[Europe/Stockholm]"` | `write the zone after a space: "2026-10-30T14:00+01:00 Europe/Stockholm"` |

- [ ] **Step 1: Write the failing tests.**
  - `ts_values` (macros), as `Ts::Fixed`:
    - `"1970-01-01T00:00Z"` → `(0, 0, 0)`;
    - `"2026-10-30T14:00:00Z"` → secs `1_793_368_800`;
    - `"2026-10-30 15:00+01:00"` → the same secs, offset `3600`;
    - `"2026-10-30T14:00-0530"` → offset `-19_800`;
    - `"2026-10-30T14:00+05:30:15"` → offset `19_815`;
    - `"1969-12-31T23:59:59.5Z"` → secs `-1`, nanos `500_000_000`.
  - `ts_errors` (macros): every row, plus the no-`tz` zone message for `"2026-10-30 14:00 Europe/Stockholm"` under `#[cfg(not(feature = "tz"))]`.
  - `days_from_civil_oracle` (macros, `jiff` dev-dep): for 20,000 random valid dates, `days_from_civil` equals jiff's `date.to_zoned(TimeZone::UTC).timestamp().as_second() / 86_400`.
  - `ts_oracle` (macros, `jiff` dev-dep): for 20,000 random date-times (any valid date, time with 0-9 fraction digits) and random offsets in ±23:59, formatted canonically, `parse_ts` gives the secs and nanos of jiff's `DateTime::to_zoned(TimeZone::fixed(offset)).timestamp()`.
  - `civil_from_days_oracle` (runtime unit test, `jiff` dev-dep): for 20,000 random days that map to years 1-9999, `civil_from_days` equals jiff's date for that Unix day.
  - `timestamp_basics` (runtime, `constlit/tests/timestamp.rs`):
    - `const T: Timestamp = ts!("2026-10-30T15:00:00.25+01:00");` has `unix_seconds() == 1_793_368_800`, `nanosecond() == 250_000_000`, `offset_seconds() == 3600`, `date() == date!("2026-10-30")` and `time() == time!("15:00:00.25")`;
    - `T.to_string() == "2026-10-30T15:00:00.25+01:00"` and `ts!("2026-10-30T14:00Z").to_string() == "2026-10-30T14:00:00Z"`;
    - `ts!("2026-10-30T14:00Z") == ts!("2026-10-30T15:00+01:00")`, with equal hashes, and `ts!("2026-10-30T14:00Z") < ts!("2026-10-30T14:00-01:00")`;
    - `Timestamp::new(0, 1_000_000_000, 0) == None` and `Timestamp::new(0, 0, 86_400) == None`.
  - `timestamp_before_epoch`: `ts!("1969-12-31T23:59:59.5Z")` has `unix_seconds() == -1`, `nanosecond() == 500_000_000`, and shows `1969-12-31T23:59:59.5Z`.
  - `timestamp_extremes`: `ts!("0001-01-01T00:00+01:00")` and `ts!("9999-12-31T23:59-01:00")` keep their written `date()`. With `jiff`, `to_jiff()` equals `jiff::Timestamp::new(secs, nanos)`. With `chrono`, `to_chrono()` succeeds.
  - `timestamp_conversions`: with `jiff`, `to_jiff()` for `T`; with `chrono`, `to_chrono().to_rfc3339() == "2026-10-30T15:00:00.250+01:00"` and `to_chrono_utc()` is 14:00 UTC; with `std`, `to_system_time()` equals `UNIX_EPOCH + Duration::new(1_793_368_800, 250_000_000)`, and `ts!("1969-12-31T23:59:59.5Z").to_system_time() == UNIX_EPOCH - Duration::from_millis(500)`.
  - UI files: `ts_missing_zone.rs` (`ts!("2026-10-30 14:00")`) and `ts_short_offset.rs` (`ts!("2026-10-30T14:00+02")`).
- [ ] **Step 2: Run** `cargo test -p constlit-macros ts && cargo test -p constlit --all-features`. Expected: FAIL.
- [ ] **Step 3: Implement.** `Timestamp::date`/`time` compute the local seconds `unix_seconds + offset_seconds`, then use `div_euclid`/`rem_euclid` by 86,400 and `civil_from_days`.
- [ ] **Step 4: Run** the Global Constraints checks and the UI snapshot review. Expected: PASS.
- [ ] **Step 5: Commit** `git commit -m "ts!: timestamps with Z or a fixed offset as constlit::Timestamp"`

---

### Task 4: named zones: `tz`, `ZonedTimestamp` and DST errors

**Files:**
- Create: `constlit/src/zoned.rs`, `constlit-macros/src/zone.rs` (`#[cfg(feature = "tz")]`)
- Modify:
  - both `Cargo.toml` files (features `tz` and `default`, optional deps, dev-deps);
  - `constlit-macros/src/ts.rs` (use the resolver when `tz` is on);
  - both `lib.rs` files (`ZonedTimestamp`, `__tzdb_version!`, `ts!` docs).
- Test:
  - unit tests in `zone.rs` and `zoned.rs`;
  - `constlit/tests/zoned.rs`;
  - `constlit/tests/ui/ts_gap.rs` and `ts_fold.rs`.

**Interfaces:**
- Consumes: `Ts`, `parse_ts`, `days_from_civil` (Task 3), `Timestamp`.
- Produces, in `constlit::ZonedTimestamp`:
  - Private fields `timestamp: Timestamp`, `zone: &'static str`.
  - `#[doc(hidden)] pub const fn __new_unchecked(timestamp: Timestamp, zone: &'static str) -> ZonedTimestamp`. There is no checked run-time constructor: one would need a tz database at run time.
  - `const fn` accessors `timestamp()`, `zone()`, `unix_seconds()`, `nanosecond()`, `offset_seconds()`, `date()` and `time()`; the last five delegate to `timestamp`.
  - Equality, ordering and hashing by instant, like `Timestamp`. `Display` as decided above.
  - With `jiff`: `to_jiff_zoned(&self) -> Result<jiff::Zoned, jiff::Error>`, which looks the zone up in jiff's run-time database. The doc says the user's jiff needs a database feature (for example `tzdb-bundle-always`), and that the result can differ from the compile-time offset if the two databases disagree.
- Produces, in `constlit-macros/src/zone.rs`: `pub fn resolve(written: &str, date: (i16, u8, u8), time: (u8, u8, u8, u32), offset: Option<i32>, zone: &str) -> Result<(i64, u32, i32), String>`, returning secs, nanos and offset. `written` is the date-time text as written (for example `"2026-10-25 02:30"`), used in messages.
- A zoned `ts!` expands to `::constlit::ZonedTimestamp::__new_unchecked(::constlit::Timestamp::__new_unchecked(secs, nanos, offset), "Europe/Stockholm")`.

**Resolution** uses jiff with the bundled database, inside the proc macro. Look the zone up with `jiff::tz::db().get(zone)`, then call `to_ambiguous_zoned` on the civil date-time:
- `Unambiguous`: use it. If an explicit offset is given and differs, report the mismatch.
- `Gap { before, after }`: always an error, even with an explicit offset. Find the transition with the first `TimeZone::following` transition after the civil time read with the `after` offset. Show its local time in the `before` and the `after` offset.
- `Fold { before, after }`: without an offset, an error showing both spellings. With an offset equal to `before` or `after`, use that one. Otherwise an error listing both.

Offsets in messages use `±HH:MM`, or `±HH:MM:SS` when the seconds aren't zero.

Error messages (exact):

| input | message |
|---|---|
| `"2026-10-30 14:00 Europe/Stokholm"` | `unknown time zone "Europe/Stokholm"; did you mean "Europe/Stockholm"?` (the suggestion only within edit distance 3; otherwise the message ends after the quoted name) |
| `"2026-10-30 14:00 PST"` | `unknown time zone "PST"; abbreviations aren't zone names, use an offset like "-08:00" or a name like "America/Los_Angeles"` (any unknown name of 2-5 uppercase letters) |
| `"2026-10-30 14:00 europe/stockholm"` | `write the zone as "Europe/Stockholm"` |
| `"2026-03-29 02:30 Europe/Stockholm"` | `2026-03-29 02:30 doesn't exist in Europe/Stockholm: the clocks jump from 02:00 to 03:00 (+01:00 to +02:00)` |
| `"2026-10-25 02:30 Europe/Stockholm"` | `2026-10-25 02:30 happens twice in Europe/Stockholm; write "2026-10-25 02:30+02:00 Europe/Stockholm" (the first) or "2026-10-25 02:30+01:00 Europe/Stockholm" (the second)` |
| `"2026-10-25 02:30+03:00 Europe/Stockholm"` | `2026-10-25 02:30 in Europe/Stockholm is +02:00 or +01:00, not +03:00` |
| `"2026-11-01 14:00+02:00 Europe/Stockholm"` | `Europe/Stockholm is at +01:00 on 2026-11-01 14:00, not +02:00; write "2026-11-01 14:00+01:00 Europe/Stockholm" or drop the offset` |

The suggested spellings keep the literal's own separator, seconds and fraction, and insert or replace only the offset.

- [ ] **Step 1: Write the failing tests.**
  - `zone_values` (macros, `#[cfg(feature = "tz")]`):
    - `"2026-10-30 14:00 Europe/Stockholm"` → `Ts::Zoned` with secs `1_793_365_200` and offset `3600`;
    - `"2026-10-25 02:30+01:00 Europe/Stockholm"` → the second pass, offset `3600`;
    - `"2026-10-30T14:00 UTC"` → offset `0`, zone `"UTC"`;
    - `"2026-10-30 14:00 Asia/Calcutta"` keeps the zone name as written, offset `19_800`.
  - `zone_errors` (macros, `#[cfg(feature = "tz")]`): every row, plus the Lord Howe fold `"2026-04-05 01:45 Australia/Lord_Howe"` naming `+11:00` and `+10:30`.
  - `zone_oracle` (macros, `#[cfg(feature = "tz")]`, `chrono-tz` dev-dep). The resolver is built on jiff, so the oracle is chrono-tz, an independent reading of the tz database. The zones are Europe/Stockholm, Europe/Dublin, America/New_York, America/St_Johns, Australia/Lord_Howe, Pacific/Apia, Asia/Kolkata and Africa/Casablanca. For every transition in 1990-2024 (listed with jiff's `TimeZone::following`), check every 15th minute from 90 minutes before to 90 minutes after its local time. Map chrono-tz's `from_local_datetime` result to an outcome: `Single` is a value, `Ambiguous` is a fold error, `None` is a gap error. The outcome and the instant must match `resolve` with no offset. For folds, `resolve` with each of chrono's two offsets must give chrono's two instants.
  - `tzdb_version` (macros, `#[cfg(feature = "tz")]`): the `__tzdb_version!` text equals `jiff_tzdb::VERSION.unwrap_or("unknown")`. Test the helper function that produces the text, not the macro.
  - `zoned_basics` (runtime, `constlit/tests/zoned.rs`):
    - `const Z: ZonedTimestamp = ts!("2026-10-30 14:00 Europe/Stockholm");` has `zone() == "Europe/Stockholm"`, `offset_seconds() == 3600`, `date() == date!("2026-10-30")` and `time() == time!("14:00")`;
    - `Z.to_string() == "2026-10-30T14:00:00+01:00 Europe/Stockholm"`;
    - `Z == ts!("2026-10-30 13:00 UTC")`, with equal hashes;
    - with `jiff`, `Z.to_jiff_zoned().unwrap() == "2026-10-30T14:00+01:00[Europe/Stockholm]".parse::<jiff::Zoned>().unwrap()`.
  - `zoned_lmt`: `ts!("1880-06-01 12:00 Europe/Paris")` has `offset_seconds() == 561` and shows `1880-06-01T12:00:00+00:09:21 Europe/Paris`; with `jiff`, `to_jiff_zoned()` has the same timestamp.
  - UI files: `ts_gap.rs` (`ts!("2026-03-29 02:30 Europe/Stockholm")`) and `ts_fold.rs` (`ts!("2026-10-25 02:30 Europe/Stockholm")`).
- [ ] **Step 2: Run** `cargo test -p constlit-macros --features tz zone && cargo test -p constlit --all-features`. Expected: FAIL.
- [ ] **Step 3: Implement.** The edit distance is a plain Levenshtein over `jiff::tz::db().available()`. The `ts!` docs in `constlit/src/lib.rs` gain:
  - `#[doc = concat!("Named zones use the IANA database bundled at compile time (version ", constlit_macros::__tzdb_version!(), ").")]`;
  - the caveat from the spec: future instants use the rules known at compile time, and a law change takes effect after a rebuild with a newer `constlit`;
  - a `compile_fail` doctest of the fold.

  Check that `cargo doc -p constlit --no-default-features` still builds.
- [ ] **Step 4: Run** the Global Constraints checks, `RUSTDOCFLAGS="-D warnings" cargo doc -p constlit --all-features --no-deps`, and the UI snapshot review. Expected: PASS.
- [ ] **Step 5: Commit** `git commit -m "ts!: named time zones resolved at compile time as constlit::ZonedTimestamp"`
