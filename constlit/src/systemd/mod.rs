//! systemd values: time spans, unit names and calendar expressions.
//!
//! ```
//! use core::time::Duration;
//! use constlit::systemd;
//!
//! const RESTART_SEC: Duration = systemd::timespan!("5min 30s");
//! assert_eq!(RESTART_SEC, Duration::from_secs(330));
//! ```

mod calendar;

pub use calendar::CalendarSpec;
#[cfg(any(feature = "jiff", feature = "chrono"))]
pub use calendar::CalendarTime;

/// A systemd time span, checked at compile time, as a [`core::time::Duration`].
///
/// systemd's syntax and units: `us`, `ms`, `s`/`sec`/`seconds`, `m`/`min`/`minutes`,
/// `h`/`hr`/`hours`, `d`/`days`, `w`/`weeks`, `month`/`months` (30.44 days) and
/// `y`/`years` (365.25 days), with decimals, combined with or without spaces:
/// `"1h 30min"`, `"1.5h"`, `"2weeks"`.
///
/// Stricter than systemd: a bare number (its unit depends on the setting) is an
/// error, as is `M` (month, easily read as `m` for minute), a value finer than 1us
/// (which systemd truncates) and `infinity`.
///
/// ```
/// use core::time::Duration;
/// assert_eq!(constlit::systemd::timespan!("1.5h"), Duration::from_secs(5400));
/// ```
///
/// ```compile_fail
/// let _ = constlit::systemd::timespan!("30"); // 30 what? write "30s"
/// ```
pub use constlit_macros::systemd_timespan as timespan;

/// A systemd unit name, checked at compile time, as a `&'static str`.
///
/// systemd's rule: `name.type` or `name@instance.type`, where the type is `service`,
/// `socket`, `device`, `mount`, `automount`, `swap`, `target`, `path`, `timer`,
/// `slice` or `scope`; a template has an empty instance (`getty@.service`). Names
/// use letters, digits and `:-_.\` (the instance may also contain `@`), and are at
/// most 255 characters.
///
/// ```
/// const WEB: &str = constlit::systemd::unit!("nginx.service");
/// const CONSOLE: &str = constlit::systemd::unit!("getty@tty1.service");
/// ```
///
/// ```compile_fail
/// let _ = constlit::systemd::unit!("nginx.servcie"); // did you mean ".service"?
/// ```
pub use constlit_macros::systemd_unit as unit;

/// A systemd calendar expression, checked at compile time, as a [`CalendarSpec`].
///
/// systemd's syntax (`systemd.time(7)`): optional weekdays (`Mon..Fri`, `Sat,Sun`),
/// an optional date `[year-]month-day` (or `month~day`, counting days from the
/// month's end), an optional time `hour:minute[:second]`, each part `*`, a value,
/// a list, a range (`1..5`) or a step (`0/15`), and the shorthands `minutely` ...
/// `semiannually`. `Display` writes systemd's normalized form.
///
/// Narrower than systemd: the only time zone is a trailing `UTC` (pass a zoned
/// date-time to `next_after` instead), and fractional seconds, `@` timestamps,
/// two-digit years, and lists after `~` are compile errors (`~07/1`, the last seven
/// days, works).
///
/// ```
/// use constlit::systemd::{self, CalendarSpec};
/// use constlit::{date, time};
/// const WORKDAYS: CalendarSpec = systemd::calendar!("Mon..Fri 09:00");
/// assert_eq!(WORKDAYS.to_string(), "Mon..Fri *-*-* 09:00:00");
/// assert!(WORKDAYS.matches(date!("2026-11-02"), time!("09:00")));
/// ```
///
/// ```compile_fail
/// let _ = constlit::systemd::calendar!("Wed 12:00 Europe/Stockholm");
/// ```
pub use constlit_macros::systemd_calendar as calendar;
