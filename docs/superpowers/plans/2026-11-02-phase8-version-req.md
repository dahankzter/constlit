# constlit Phase 8 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `version_req!(">=1.2, <2")`, producing `constlit::VersionReq`, with Cargo's (the `semver` crate's) syntax and matching semantics.

**Architecture:**
- `constlit-macros/src/version_req.rs` parses the requirement into comparators (op, major, optional minor and patch, pre-release). It reuses `version.rs`'s identifier rules for the pre-release.
- The macro emits a `&'static [Comparator]` and the literal.
- The runtime type `constlit/src/version_req.rs` implements `matches(&Version)` exactly as `semver` 1.x does, including its pre-release rule.
- Both sides are checked against the `semver` crate: parsing structurally on generated inputs, and matching on random versions.

**Tech Stack:** As before. The `semver` dev-dependency already exists in both crates; the runtime `semver` feature gains `VersionReq::to_semver`.

**Spec:** `docs/superpowers/specs/2026-11-02-constlit-0.3-design.md`, Phase 8.

**Starting point:** `main` after phase 7.
- `constlit-macros/src/version.rs` (`parse_version`, `check_identifiers`);
- `constlit/src/version.rs` (`Version` with `major()`, `minor()`, `patch()`, `pre() -> &'static str`, and a private `cmp_pre`).

## Global Constraints

The phase 7 list, unchanged.

## Decisions this plan makes (my calls, within the spec)

- **Syntax: whatever `semver::VersionReq::parse` accepts, nothing more.** The parser test compares our acceptance and our comparators (`op`, `major`, `minor`, `patch`, `pre`) with semver's on generated inputs. Two exceptions:
  - leading or trailing whitespace is our usual error;
  - `""` is `empty version requirement; write e.g. "^1.2" or ">=1.2, <2"`.
- **Ops:** the enum `constlit::Op` has `Exact`, `Greater`, `GreaterEq`, `Less`, `LessEq`, `Tilde`, `Caret` and `Wildcard`, mirroring `semver::Op`.
  - `"*"` (and `"x"`, `"X"`) is a requirement with no comparators, as semver parses it.
  - `"1.*"` is a `Wildcard` with no minor.
- **Matching is semver 1.x's algorithm,** written out in the plan below so the semantics are fixed. `Version`'s `cmp_pre` becomes `pub(crate)` for it.
- **Equality.** `VersionReq` compares and hashes by its comparators, so `">=1.2"` equals `">= 1.2"`. `Display` writes the literal. `to_semver()` exists under the `semver` feature.
- **Comparator** is a public type with private fields and `const fn` accessors: `op()`, `major()`, `minor() -> Option<u64>`, `patch() -> Option<u64>`, `pre() -> &'static str`.

Matching (semver 1.x, `matches_req`); every comparator must match:
- **exact:** majors equal, the minor (if any) and patch (if any) equal, and the pre-releases equal;
- **greater** (less is mirrored): if the majors differ, compare them; with no minor, false; if the minors differ, compare them; with no patch, false; if the patches differ, compare them; otherwise pre-release `>`;
- **`GreaterEq`/`LessEq`:** exact or greater/less;
- **tilde:** majors equal; if the comparator has a minor, the minors are equal; if it has a patch and the patches differ, the version's patch `>`; otherwise pre-release `>=`;
- **caret:**
  1. The majors must be equal; with no minor, true.
  2. With no patch: if the comparator's major is above 0, the version's minor must be `>=`; otherwise the minors must be equal.
  3. With a patch:
     - if the major is above 0: if the minors differ, the version's minor `>`; else if the patches differ, the version's patch `>`;
     - else if the minor is above 0: if the minors differ, false; else if the patches differ, the version's patch `>`;
     - else, if either the minor or the patch differs, false.

     Otherwise pre-release `>=`.
- **wildcard:** as exact, but only over the parts given.

Then, if the version has a pre-release, at least one comparator must have the same major, minor (`Some`) and patch (`Some`) and a non-empty pre-release.

Pre-release order is SemVer precedence, where no pre-release ranks above any pre-release (as `Version` already orders).

## Review Focus

1. **Pre-release opt-in.**
   - `version_req!(">=1.2.3")` does not match `1.3.0-alpha`.
   - `">=1.2.3-alpha"` matches `1.2.3-beta` but not `1.2.4-alpha`.
2. **Caret at zero.**
   - `"^0.2.3"` matches `0.2.4` but not `0.3.0`.
   - `"^0.0.3"` matches only `0.0.3`.
   - `"^0"` matches any `0.x.y`.
3. **Wildcards.** `"1.*"`, `"1.2.*"`, `"*"` and `"1.x"` agree with semver.
4. **Several comparators.** `">=1.2, <1.5"`, with and without spaces, and up to semver's comparator limit.
5. **Equality across spellings.** `version_req!(">=1.2") == version_req!(">= 1.2")`, while `Display` keeps each.

The parsing items are pinned by `version_req_oracle` (Task 1); the matching items by `version_req_matches` (Task 2).

---

### Task 1: Parser and `version_req!` expansion

**Files:**
- Create: `constlit-macros/src/version_req.rs`.
- Modify: `constlit-macros/src/version.rs` (make the pre-release check reusable) and `constlit-macros/src/lib.rs` (`#[proc_macro] pub fn version_req`).
- Test: unit tests in `version_req.rs`, and the UI test `constlit/tests/ui/version_req_space.rs`.

**Interfaces:**
- Produces:
  - `parse_version_req(s: &str) -> Result<Vec<Comparator>, String>`;
  - `pub struct Comparator { pub op: &'static str /* "=", ">", ">=", "<", "<=", "~", "^", "*" */, pub major: u64, pub minor: Option<u64>, pub patch: Option<u64>, pub pre: String }`.
- The expansion is `::constlit::VersionReq::__new_unchecked(&[::constlit::Comparator::__new_unchecked(::constlit::Op::Caret, 1u64, Some(2u64), None, ""), …], "<literal>")`.

- [ ] **Step 1: Failing tests.**
  - `version_req_values`: the comparators of:
    - `"1.2"` (caret 1, 2, None);
    - `"^1.2.3"`, `"~1.2"`, `"=1.2.3-alpha.1"`, `">=1.2, <2"`, `">= 1.2 , < 2"`;
    - `"1.*"` (wildcard 1, None, None), `"1.2.x"`;
    - `"*"` (no comparators);
    - `"<1.2.3-rc.1"`.
  - `version_req_errors`:
    - `""`;
    - `" ^1"` (spaces);
    - and the exact messages for `">=1.2 <2"`, `"^1.2.3.4"`, `"1.2.3-"`, `">=v1.2"`, `"^01.2"` and `"=*"`, taken from semver's error kinds and reworded. Each message names the problem and, where one exists, gives a corrected literal, which the test checks is valid.
  - `version_req_oracle`: 20,000 generated requirements, seed 8, from 1-3 comparators. Each comparator draws:
    - an op from `["", "=", ">", ">=", "<", "<=", "~", "^", "=>", "!"]`;
    - optional spaces;
    - 1-3 numeric parts from `0`, `1`, `2`, `10`, `01`, `*`, `x`, `X` or `a`;
    - an optional `-alpha.1` or `+build`.

    Comparators are joined by `,`, `, ` or ` `. For each:
    - `semver::VersionReq::parse` and ours agree on acceptance, except for our whitespace class;
    - for accepted inputs, every comparator's `(op, major, minor, patch, pre)` is equal;
    - every suggestion parses;
    - at least 5,000 accepted.
  - UI: `version_req_space.rs` (`">=1.2 <2"`).
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the checks. Expected: PASS.
- [ ] **Step 5: Commit** `version_req!: Cargo version requirements, parsed like the semver crate`.

---

### Task 2: `VersionReq`, `Comparator`, matching and `to_semver`

**Files:**
- Create: `constlit/src/version_req.rs`.
- Modify: `constlit/src/version.rs` (`cmp_pre` becomes `pub(crate)`) and `constlit/src/lib.rs` (`mod version_req; pub use version_req::{Comparator, Op, VersionReq};` and the documented re-export `pub use constlit_macros::version_req;`).
- Test: `constlit/tests/version_req.rs`.

**Interfaces:** As in Decisions, plus:
- `VersionReq::comparators(&self) -> &'static [Comparator]`;
- `VersionReq::as_str()`;
- `VersionReq::matches(&self, version: &Version) -> bool`, a plain `fn`;
- `VersionReq::to_semver(&self) -> semver::VersionReq`, under `feature = "semver"`.

- [ ] **Step 1: Failing tests.**
  - `version_req_matches`: a `macro_rules!` lists about 40 requirement literals covering every op, the zero carets, wildcards, pre-releases and multiple comparators. Each one is built twice, with `version_req!(lit)` and `semver::VersionReq::parse(lit)`.

    For 2,000 random versions (SplitMix64; major, minor and patch in `0..=3`; the pre-release `""` or one of `"alpha"`, `"alpha.1"`, `"beta"` or `"rc.1"`), built with `Version::__new_unchecked` and the matching `semver::Version`, `ours.matches(&v) == theirs.matches(&sv)`.
  - `version_req_accessors`:
    - `const R: VersionReq = version_req!(">=1.2, <2");` gives `R.comparators().len() == 2`, `R.comparators()[0].op() == Op::GreaterEq` and `R.to_string() == ">=1.2, <2"`;
    - `version_req!(">=1.2") == version_req!(">= 1.2")`, with equal hashes;
    - `version_req!("*").comparators().is_empty()`;
    - a `static`.
  - The Review Focus cases as named asserts.
  - `to_semver` (feature `semver`): `R.to_semver() == semver::VersionReq::parse(">=1.2, <2").unwrap()`.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement** the matching exactly as written in Decisions. The re-export has docs and a doctest.
- [ ] **Step 4: Run** the checks. Expected: PASS.
- [ ] **Step 5: Commit** `VersionReq: semver-compatible matching for version_req!`.
