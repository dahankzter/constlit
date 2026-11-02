# constlit Phase 9 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the `systemd` module: `timespan!` (`core::time::Duration`), `unit!` (`&'static str`) and `calendar!` (`systemd::CalendarSpec`, with systemd's normalized form, `matches`, and `next_after` for jiff and chrono).

**Architecture:**
- The parsers live in `constlit-macros/src/systemd_*.rs`, re-exported from `constlit::systemd`.
- The oracle is `systemd-analyze` (systemd 262). The script `constlit-macros/oracle/systemd_fixtures.py` runs one expression per process (systemd leaves out "Original form" when the input is already normalized, and stops at failures). It writes TSV fixtures to `constlit-macros/src/testdata/systemd_*.tsv`.
- `CalendarSpec` holds bit sets for matching, plus the normalized form as a `&'static str` computed by the macro. `Display` is then exact, without re-deriving systemd's formatting at run time.

**Tech Stack:** As before. `systemd-analyze` is needed only to regenerate fixtures; the headers record `systemd-analyze --version`.

**Spec:** `docs/superpowers/specs/2026-11-02-constlit-0.3-design.md`, Phase 9.

## Global Constraints

The phase 8 list, plus: every older fixture regenerates byte for byte.

## Decisions this plan makes (my calls, within the spec)

**Time spans.** These were probed while writing the plan:
- systemd accepts surrounding spaces, a bare number (as seconds), `M` (month) and digits past µs (truncated, so `1.0000001s` is 1 s).
- Ours rejects each, with a suggestion. The oracle test allows exactly these classes:
  - `"30"` → `missing unit after 30; write e.g. "30s" or "30min"`;
  - `"1M"` → `"M" means month here and reads like "m" (minute); write "1month" or "1min"`;
  - finer than 1 µs → `"1.0000001s" is finer than 1us, the smallest step systemd keeps`;
  - `"infinity"` → `infinity has no Duration; use Duration::MAX`.

**Units:** `units` comes from systemd's `parse_time` unit table, and is pinned by the fixture.

**Unit names.**
- systemd's `unit_name_is_valid` rules from the spec.
- The instance may contain `@`, as systemd's `VALID_CHARS_WITH_AT` allows.
- There is no CLI oracle, so a unit-test table covers the cases of systemd's `test-unit-name.c`, as the spec says.

**Calendar scope:**
- `@epoch` timestamps, fractional seconds and IANA zone suffixes are errors naming the limit. `UTC` is the only suffix.
- Lower-case weekday names are accepted, as systemd accepts them, and normalize to `Mon`.
- Years run 1970-2199, systemd's range.

**Calendar semantics:**
- `next_after(&Zoned)` evaluates in the date-time's zone, or in UTC for a `UTC` spec.
- **Time skipped when the clocks go forward:** the matching local time is skipped.
- **Time repeated when the clocks go back:** it fires once, at the first occurrence.
- Both behaviours were observed with `TZ=Europe/Stockholm` (2026-03-29 and 2026-10-25), and the fixture pins them with base times around DST changes in Stockholm, New York and Lord Howe.

**`CalendarSpec` layout:**
- Bit sets for the weekdays (7), months (12), days (31, plus a `from_end` flag for `~`), hours (24), minutes (60) and seconds (60).
- Years as a bit set over 1970-2199, in four `u64`s.
- `utc: bool`, and `normalized: &'static str`.

## Review Focus

1. **The `~` days.**
   - `*-02~03` is the third-last day of February, which differs in leap years.
   - `*-*~07/1` means the last 7 days of each month, every day.
   - Pinned by the calendar fixture (Task 4).
2. **DST.** The skip and once-at-first-fold rule in Stockholm, plus Lord Howe's 30-minute DST. Pinned by the next-elapse fixture (Task 4).
3. **Normalization.** Shorthands (`quarterly` → `*-01,04,07,10-01 00:00:00`), padding, repetitions (`*:0/15` → `*-*-* *:00/15:00`), and weekday ranges and lists. Pinned by the normalized-form fixture (Task 3).
4. **Time span units by case.** `1m` is a minute, `1M` is an error, and `1h30min` equals `1.5h`. Pinned by the timespan fixture (Task 1).
5. **Templates and instances.**
   - `getty@.service` and `getty@tty1.service` are valid.
   - `@.service` and `foo.servcie` are errors, the latter with "did you mean". Pinned by `unit_errors` (Task 2).

---

### Task 1: `systemd` module and `timespan!`

**Files:**
- Create: `constlit/src/systemd/mod.rs`, `constlit-macros/src/systemd_timespan.rs`, `constlit-macros/oracle/systemd_fixtures.py` (the `timespan` subcommand) and `src/testdata/systemd_timespan.tsv`.
- Modify: `constlit/src/lib.rs` (`pub mod systemd;`) and `constlit-macros/src/lib.rs` (`systemd_timespan`).
- Test: unit tests, `constlit/tests/systemd.rs`, UI test `constlit/tests/ui/timespan_bare.rs`.

**Interfaces:** `parse_timespan(s: &str) -> Result<(u64 /*secs*/, u32 /*nanos*/), String>`. The expansion is `::core::time::Duration::new(secs, nanos)`.

- [ ] **Step 1: Write the fixture script.**
  - Inputs: a fixed list (every unit spelling, with and without spaces; the decisions above; `"1h 30min"`, `"2h30m"`, `"1w2d"`, `"1.5 hours"`, `"500ms"`, `"0"`, `"1h,30min"`) and 2,000 generated inputs, seed 9. Each has 1-3 components drawn from numbers (`0`, `1`, `2`, `1.5`, `0.25`, `10`, `007`) and units, with optional spaces.
  - Each line is `input<TAB>ok<TAB><µs>` or `input<TAB>err`, from `systemd-analyze timespan`.
- [ ] **Step 2: Failing tests.**
  - `timespan_values`: `"5min 30s"` → 330 s, `"1.5h"`, `"2 weeks"`, `"1month"` (2,629,800 s), `"1y"`, `"500ms"` and `"1us"`.
  - `timespan_errors`: the decisions above, plus `"5 parsecs"` (unknown unit, with "did you mean" where one is close) and `""`.
  - `timespan_oracle`:
    - agreement both ways, with the allowances above checked against the input;
    - µs equality;
    - every suggestion parses;
    - at least 1,000 accepted.
  - Runtime: a `const`, a `static`, and `timespan!("1h30min") == Duration::from_secs(5400)`.
- [ ] **Step 3: Run.** Expected: FAIL. **Step 4: Implement.** **Step 5: Run the checks.** **Step 6: Commit** `systemd::timespan!: systemd time spans as Duration`.

---

### Task 2: `unit!`

**Files:**
- Create: `constlit-macros/src/systemd_unit.rs`.
- Modify: `constlit/src/systemd/mod.rs` and `constlit-macros/src/lib.rs`.
- Test: unit tests and the UI test `unit_type.rs`.

**Error messages:**

| input | message |
|---|---|
| `""` | `empty unit name; write e.g. "nginx.service"` |
| `"nginx"` | `missing unit type; write e.g. "nginx.service"` |
| `"nginx.servcie"` | `unknown unit type ".servcie"; did you mean ".service"?` |
| `"nginx.daemon"` | `unknown unit type ".daemon"; use service, socket, device, mount, automount, swap, target, path, timer, slice or scope` |
| `".service"`, `"@.service"`, `"@x.service"` | `missing unit name before "."` (before `"@"` for the `@` forms) |
| `"ng/inx.service"` | ``invalid character `/` in a unit name`` |
| 256 characters | `unit names are at most 255 characters; this one has 256` |

- [ ] **Steps:**
  1. Failing tests: valid names `foo.service`, `foo@.service`, `foo@bar.service`, `foo@bar@baz.service`, `a.b-c_d:e\x2d.mount`, `-.slice`, and one name for each unit type. Invalid: the table, plus `foo@bar.` and `foo.service.`.
  2. Run. Expected: FAIL.
  3. Implement.
  4. Run the checks.
  5. Commit `systemd::unit!: systemd unit names`.

---

### Task 3: `calendar!`: parsing and normalization

**Files:**
- Create: `constlit-macros/src/systemd_calendar.rs` and `src/testdata/systemd_calendar.tsv`.
- Modify: `systemd_fixtures.py` (`calendar` subcommand).
- Test: unit tests.

**Interfaces:**
- Produces `parse_calendar(s: &str) -> Result<Calendar, String>`, where `Calendar { weekdays: u8, years: [u64; 4], months: u16, days: u32, from_end: bool, hours: u32, minutes: u64, seconds: u64, utc: bool, normalized: String }`. The bits index values from 0 (or from 1970 for years; days and months from 1).

- [ ] **Step 1: Fixture.**
  - Inputs: the expressions probed while writing this plan (shorthands, weekday lists and ranges, `~`, repetitions, ranges, lists, `UTC`, the IANA, fraction and `@` exclusions, out-of-range values), plus 2,000 generated inputs, seed 10. Each is built from optional weekdays, an optional date and an optional time, each part `*`, a value, a list, a range or a repetition.
  - Each line is `input<TAB>ok<TAB><normalized form>` or `input<TAB>err`, run under `TZ=UTC` with `--base-time` set.
- [ ] **Step 2: Failing tests.**
  - `calendar_values`: the bit sets for `"Mon..Fri *-*-* 09:00"`, `"quarterly"` and `"*-02~03"`.
  - `calendar_errors`: an IANA zone (`calendar! takes "UTC" only; for a time zone, call next_after with a zoned date-time`), a fraction, `@epoch`, `"Sat..Mon"` (`weekday ranges go forward; write "Sat,Sun,Mon"`), `"25:00"` and `""`.
  - `calendar_oracle`:
    - acceptance agrees, except the declared exclusions;
    - our normalized form equals systemd's on every accepted line;
    - at least 1,000 accepted.
- [ ] **Step 3: Run.** Expected: FAIL. **Step 4: Implement** systemd's normalization: sort and deduplicate each chain, zero-pad, keep ranges and repetitions as written, and use the shorthand expansions. The fixture decides every detail. **Step 5: Run the checks.** **Step 6: Commit** `systemd calendar parser with systemd's normalized form`.

---

### Task 4: `CalendarSpec`, `matches` and `next_after`

**Files:**
- Create: `constlit/src/systemd/calendar.rs` and `src/testdata/systemd_elapse.tsv`.
- Modify: `systemd_fixtures.py` (`elapse` subcommand), `constlit/src/systemd/mod.rs` and `constlit-macros/src/lib.rs` (`systemd_calendar`).
- Test: `constlit/tests/systemd_calendar.rs`.

**Interfaces:**
- `CalendarSpec`, with private fields as in Decisions;
- `matches(&self, date: Date, time: TimeOfDay) -> bool`, using constlit's own date and time types;
- `Display` writes the normalized form;
- with `jiff`: `next_after(&self, after: &jiff::Zoned) -> Option<jiff::Zoned>`;
- with `chrono`: `next_after_chrono<Tz: TimeZone>(&self, after: &DateTime<Tz>) -> Option<DateTime<Tz>>`, mirroring `CronSpec`'s chrono entry point (check cron_time.rs for the exact pattern and reuse it).

- [ ] **Step 1: Fixture `elapse`.**
  - About 60 expressions (the Task 3 fixed list's valid ones) times base times. The bases are three ordinary days, plus the days before the DST changes in Europe/Stockholm, America/New_York and Australia/Lord_Howe for 2026.
  - Each runs under `TZ` set to `UTC` or the zone, with `--iterations=5`.
  - Each line is `tz<TAB>base_unix<TAB>expr<TAB>e1,e2,…`, the next elapses as Unix seconds taken from the "(in UTC)" lines (or the main lines under `TZ=UTC`), or `never`.
- [ ] **Step 2: Failing tests.**
  - Runtime `calendar_elapse`, under the `jiff` feature: for every fixture line, the expression is compiled through a `macro_rules!` list of the same literals. Starting from the base time in the zone, calling `next_after` five times gives the fixture's elapses.
  - `calendar_matches`: a few hand cases.
  - `calendar_chrono`: a sample of lines through the chrono entry point.
  - `Display` equals the normalized form.
- [ ] **Step 3: Run.** Expected: FAIL. **Step 4: Implement.** Search forward by fields: year, then month, day, hour, minute and second, using the bit sets. Respect `from_end` days and weekdays. Convert each candidate civil time to the zone: skip it if it doesn't exist, and take the earlier instant if it is ambiguous. Return `None` after 2199. **Step 5: Run the checks.** **Step 6: Commit** `systemd::calendar!: CalendarSpec with next_after matching systemd`.
