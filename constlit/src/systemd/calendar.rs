//! [`CalendarSpec`], the value of `systemd::calendar!`.

use core::fmt;

use crate::calendar::{days_in_month, weekday};
use crate::{Date, TimeOfDay};

/// Years `calendar!` covers, as systemd does.
const FIRST_YEAR: i32 = 1970;
const LAST_YEAR: i32 = 2199;

/// A systemd calendar expression, usually written with [`calendar!`](crate::systemd::calendar).
///
/// `Display` writes systemd's normalized form (what `systemd-analyze calendar`
/// prints). With the `jiff` or `chrono` feature, `next_after`
/// finds the next elapse as systemd does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CalendarSpec {
    weekdays: u8,
    years: [u64; 4],
    months: u16,
    days: u32,
    from_end: bool,
    hours: u32,
    minutes: u64,
    seconds: u64,
    utc: bool,
    normalized: &'static str,
}

/// A civil date and time, to the second.
#[cfg(any(feature = "jiff", feature = "chrono"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Civil {
    pub(crate) year: i32,
    pub(crate) month: u8,
    pub(crate) day: u8,
    pub(crate) hour: u8,
    pub(crate) minute: u8,
    pub(crate) second: u8,
}

impl CalendarSpec {
    #[doc(hidden)]
    #[allow(clippy::too_many_arguments)]
    pub const fn __new_unchecked(
        weekdays: u8,
        years: [u64; 4],
        months: u16,
        days: u32,
        from_end: bool,
        hours: u32,
        minutes: u64,
        seconds: u64,
        utc: bool,
        normalized: &'static str,
    ) -> Self {
        Self {
            weekdays,
            years,
            months,
            days,
            from_end,
            hours,
            minutes,
            seconds,
            utc,
            normalized,
        }
    }

    /// Whether the expression ends in `UTC`, so it runs on UTC time in any zone.
    pub const fn is_utc(&self) -> bool {
        self.utc
    }

    /// Whether the expression matches this date and time (to the second): local
    /// time, or UTC for a `UTC` expression.
    pub const fn matches(&self, date: Date, time: TimeOfDay) -> bool {
        self.date_matches(date.year() as i32, date.month(), date.day())
            && self.hours & (1 << time.hour()) != 0
            && self.minutes & (1 << time.minute()) != 0
            && self.seconds & (1 << time.second()) != 0
    }

    const fn date_matches(&self, year: i32, month: u8, day: u8) -> bool {
        if year < FIRST_YEAR || year > LAST_YEAR {
            return false;
        }
        let i = (year - FIRST_YEAR) as usize;
        if self.years[i / 64] & (1 << (i % 64)) == 0 || self.months & (1 << month) == 0 {
            return false;
        }
        let day_bit = if self.from_end {
            days_in_month(year, month) - day + 1
        } else {
            day
        };
        // Weekday bits run Monday (0) to Sunday (6); `weekday` counts from Sunday.
        let weekday_bit = (weekday(year, month, day) + 6) % 7;
        self.days & (1 << day_bit) != 0 && self.weekdays & (1 << weekday_bit) != 0
    }

    /// The first matching civil time at or after `from`, if any before 2200.
    #[cfg(any(feature = "jiff", feature = "chrono"))]
    pub(crate) fn next_civil(&self, from: Civil) -> Option<Civil> {
        let mut year = from.year.max(FIRST_YEAR);
        let mut floor = if from.year < FIRST_YEAR {
            Civil {
                year,
                month: 1,
                day: 1,
                hour: 0,
                minute: 0,
                second: 0,
            }
        } else {
            from
        };
        while year <= LAST_YEAR {
            let first_month = if year == floor.year { floor.month } else { 1 };
            for month in first_month..=12 {
                let on_floor = year == floor.year && month == floor.month;
                let first_day = if on_floor { floor.day } else { 1 };
                for day in first_day..=days_in_month(year, month) {
                    if !self.date_matches(year, month, day) {
                        continue;
                    }
                    let on_floor = on_floor && day == floor.day;
                    if let Some((hour, minute, second)) = self.time_from(on_floor, floor) {
                        return Some(Civil {
                            year,
                            month,
                            day,
                            hour,
                            minute,
                            second,
                        });
                    }
                }
            }
            year += 1;
            floor = Civil {
                year,
                month: 1,
                day: 1,
                hour: 0,
                minute: 0,
                second: 0,
            };
        }
        None
    }

    /// The first matching time of day, at or after `floor`'s time when `on_floor`.
    #[cfg(any(feature = "jiff", feature = "chrono"))]
    fn time_from(&self, on_floor: bool, floor: Civil) -> Option<(u8, u8, u8)> {
        let first_hour = if on_floor { floor.hour } else { 0 };
        for hour in first_hour..24 {
            if self.hours & (1 << hour) == 0 {
                continue;
            }
            let on_floor = on_floor && hour == floor.hour;
            let first_minute = if on_floor { floor.minute } else { 0 };
            for minute in first_minute..60 {
                if self.minutes & (1 << minute) == 0 {
                    continue;
                }
                let first_second = if on_floor && minute == floor.minute {
                    floor.second
                } else {
                    0
                };
                if let Some(second) = (first_second..60).find(|s| self.seconds & (1 << s) != 0) {
                    return Some((hour, minute, second));
                }
            }
        }
        None
    }

    /// The next elapse strictly after `t`, as systemd computes it: in `t`'s time
    /// zone (or UTC for a `UTC` expression), skipping local times the clocks jump
    /// over, and taking the first occurrence after `t` of a local time that happens
    /// twice. `None` if nothing matches before the year 2200.
    #[cfg(any(feature = "jiff", feature = "chrono"))]
    #[cfg_attr(docsrs, doc(cfg(any(feature = "jiff", feature = "chrono"))))]
    pub fn next_after<T: CalendarTime>(&self, t: &T) -> Option<T> {
        T::next_for(self, t)
    }
}

#[cfg(any(feature = "jiff", feature = "chrono"))]
impl Civil {
    /// One second later.
    pub(crate) fn next_second(self) -> Civil {
        let mut c = self;
        c.second += 1;
        if c.second == 60 {
            c.second = 0;
            c.minute += 1;
            if c.minute == 60 {
                c.minute = 0;
                c.hour += 1;
                if c.hour == 24 {
                    c.hour = 0;
                    c.day += 1;
                    if c.day > days_in_month(c.year, c.month) {
                        c.day = 1;
                        c.month += 1;
                        if c.month == 13 {
                            c.month = 1;
                            c.year += 1;
                        }
                    }
                }
            }
        }
        c
    }
}

impl fmt::Display for CalendarSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.normalized)
    }
}

/// Date-time types [`CalendarSpec::next_after`] works with: `jiff::Zoned` and
/// chrono's `DateTime<Tz>`.
#[cfg(any(feature = "jiff", feature = "chrono"))]
pub trait CalendarTime: Sized + sealed::Sealed {}

#[cfg(any(feature = "jiff", feature = "chrono"))]
mod sealed {
    use super::CalendarSpec;

    pub trait Sealed: Sized {
        fn next_for(spec: &CalendarSpec, t: &Self) -> Option<Self>;
    }
}

#[cfg(any(feature = "jiff", feature = "chrono"))]
impl<T: sealed::Sealed> CalendarTime for T {}

#[cfg(feature = "jiff")]
mod with_jiff {
    use jiff::Zoned;
    use jiff::civil::DateTime;
    use jiff::tz::{AmbiguousOffset, TimeZone};

    use super::{CalendarSpec, Civil, sealed::Sealed};

    impl Sealed for Zoned {
        fn next_for(spec: &CalendarSpec, t: &Self) -> Option<Self> {
            let zone = if spec.utc {
                TimeZone::UTC
            } else {
                t.time_zone().clone()
            };
            let local = t.with_time_zone(zone.clone()).datetime();
            let mut from = Civil {
                year: i32::from(local.year()),
                month: local.month() as u8,
                day: local.day() as u8,
                hour: local.hour() as u8,
                minute: local.minute() as u8,
                second: local.second() as u8,
            }
            .next_second();
            loop {
                let c = spec.next_civil(from)?;
                let civil = DateTime::new(
                    c.year as i16,
                    c.month as i8,
                    c.day as i8,
                    c.hour as i8,
                    c.minute as i8,
                    c.second as i8,
                    0,
                )
                .ok()?;
                let instants = match zone.to_ambiguous_timestamp(civil).offset() {
                    AmbiguousOffset::Unambiguous { offset } => [Some(offset), None],
                    AmbiguousOffset::Gap { .. } => [None, None],
                    AmbiguousOffset::Fold { before, after } => [Some(before), Some(after)],
                };
                // The first instant of this local time that is after `t`.
                for offset in instants.into_iter().flatten() {
                    if let Ok(ts) = offset.to_timestamp(civil)
                        && ts > t.timestamp()
                    {
                        return Some(ts.to_zoned(t.time_zone().clone()));
                    }
                }
                from = c.next_second();
            }
        }
    }
}

#[cfg(feature = "chrono")]
mod with_chrono {
    use chrono::{DateTime, Datelike, LocalResult, NaiveDate, TimeZone, Timelike, Utc};

    use super::{CalendarSpec, Civil, sealed::Sealed};

    impl<Tz: TimeZone> Sealed for DateTime<Tz> {
        fn next_for(spec: &CalendarSpec, t: &Self) -> Option<Self> {
            let local = if spec.utc {
                t.naive_utc()
            } else {
                t.naive_local()
            };
            let mut from = Civil {
                year: local.year(),
                month: local.month() as u8,
                day: local.day() as u8,
                hour: local.hour() as u8,
                minute: local.minute() as u8,
                second: local.second() as u8,
            }
            .next_second();
            loop {
                let c = spec.next_civil(from)?;
                let naive = NaiveDate::from_ymd_opt(c.year, c.month.into(), c.day.into())?
                    .and_hms_opt(c.hour.into(), c.minute.into(), c.second.into())?;
                let candidates = if spec.utc {
                    LocalResult::Single(Utc.from_utc_datetime(&naive).with_timezone(&t.timezone()))
                } else {
                    t.timezone().from_local_datetime(&naive)
                };
                // The first instant of this local time that is after `t`.
                let found = match candidates {
                    LocalResult::Single(x) => [Some(x), None],
                    LocalResult::Ambiguous(a, b) if a <= b => [Some(a), Some(b)],
                    LocalResult::Ambiguous(a, b) => [Some(b), Some(a)],
                    LocalResult::None => [None, None],
                };
                if let Some(x) = found.into_iter().flatten().find(|x| x > t) {
                    return Some(x);
                }
                from = c.next_second();
            }
        }
    }
}
