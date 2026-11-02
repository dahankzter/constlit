use core::fmt;

use crate::calendar::{Minute, weekday};

/// A cron schedule, usually made with [`cron!`](crate::cron).
///
/// It holds one bit per allowed value in each of the five fields. Equality is
/// structural and includes the source text, so `cron!("@daily")` and
/// `cron!("0 0 * * *")` are not equal although they match the same minutes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CronSpec {
    source: &'static str,
    minutes: u64,
    hours: u32,
    days: u32,
    months: u16,
    weekdays: u8,
    dom_star: bool,
    dow_star: bool,
    /// The minute or hour field starts with `*`: the schedule runs on real time
    /// when the clocks go back.
    pub(crate) time_star: bool,
    /// Set by [`catch_up_gaps`](Self::catch_up_gaps).
    pub(crate) catch_up_gaps: bool,
}

impl CronSpec {
    #[doc(hidden)]
    #[allow(clippy::too_many_arguments)]
    pub const fn __new_unchecked(
        source: &'static str,
        minutes: u64,
        hours: u32,
        days: u32,
        months: u16,
        weekdays: u8,
        dom_star: bool,
        dow_star: bool,
        time_star: bool,
    ) -> Self {
        Self {
            source,
            minutes,
            hours,
            days,
            months,
            weekdays,
            dom_star,
            dow_star,
            time_star,
            catch_up_gaps: false,
        }
    }

    /// This schedule, but in [`next_after`](Self::next_after) every time skipped by
    /// a daylight-saving gap runs once, at the first instant after the gap.
    ///
    /// By default only schedules with fixed minute and hour fields catch up, as in
    /// Vixie cron: `30 * * * *` skips 02:30 on the night the clocks jump from 02:00
    /// to 03:00, while `cron!("30 * * * *").catch_up_gaps()` runs at 03:00 instead.
    /// The repeated hour when the clocks go back is unaffected.
    ///
    /// ```
    /// use constlit::{CronSpec, cron};
    /// const BILLING: CronSpec = cron!("30 * * * *").catch_up_gaps();
    /// assert_ne!(BILLING, cron!("30 * * * *"));
    /// ```
    pub const fn catch_up_gaps(self) -> Self {
        Self {
            catch_up_gaps: true,
            ..self
        }
    }

    /// The expression as written, like `"*/5 * * * *"` or `"@daily"`.
    pub const fn as_str(&self) -> &'static str {
        self.source
    }

    /// Bit m is set when minute m (0-59) matches.
    pub const fn minutes(&self) -> u64 {
        self.minutes
    }

    /// Bit h is set when hour h (0-23) matches.
    pub const fn hours(&self) -> u32 {
        self.hours
    }

    /// Bit d is set when day d (1-31) matches.
    pub const fn days_of_month(&self) -> u32 {
        self.days
    }

    /// Bit m is set when month m (1-12) matches.
    pub const fn months(&self) -> u16 {
        self.months
    }

    /// Bit w is set when weekday w (0-6, Sunday = 0) matches.
    pub const fn days_of_week(&self) -> u8 {
        self.weekdays
    }

    /// Whether the schedule fires at this minute. `weekday` is 0-6 with Sunday = 0;
    /// out-of-range arguments give `false`.
    ///
    /// The day fields follow Vixie cron: when either field starts with `*`, both
    /// must match; when both are restricted, either one is enough.
    pub const fn matches(&self, minute: u8, hour: u8, day: u8, month: u8, weekday: u8) -> bool {
        if minute > 59
            || hour > 23
            || day == 0
            || day > 31
            || month == 0
            || month > 12
            || weekday > 6
        {
            return false;
        }
        let time = self.minutes & 1 << minute != 0
            && self.hours & 1 << hour != 0
            && self.months & 1 << month != 0;
        let day_ok = self.days & 1 << day != 0;
        let weekday_ok = self.weekdays & 1 << weekday != 0;
        let date = if self.dom_star || self.dow_star {
            day_ok && weekday_ok
        } else {
            day_ok || weekday_ok
        };
        time && date
    }
}

impl CronSpec {
    /// The first matching minute strictly after `t`, or `None` when nothing matches
    /// within 400 years. The Gregorian calendar repeats every 400 years, so that
    /// means the schedule never matches, like `0 0 30 2 *`.
    #[cfg_attr(not(any(feature = "jiff", feature = "chrono")), allow(dead_code))]
    pub(crate) fn next_civil_after(&self, t: Minute) -> Option<Minute> {
        let end = t.year.checked_add(400)?;
        let mut m = t.next();
        while m.year <= end {
            let day_ok = self.days & 1 << m.day != 0;
            let weekday_ok = self.weekdays & 1 << weekday(m.year, m.month, m.day) != 0;
            let date_ok = if self.dom_star || self.dow_star {
                day_ok && weekday_ok
            } else {
                day_ok || weekday_ok
            };
            m = if self.months & 1 << m.month == 0 {
                m.next_month()
            } else if !date_ok {
                m.next_day()
            } else if self.hours & 1 << m.hour == 0 {
                m.next_hour()
            } else if self.minutes & 1 << m.minute == 0 {
                m.next()
            } else {
                return Some(m);
            };
        }
        None
    }

    /// The first time this schedule fires strictly after `t`, in `t`'s own type
    /// and, for zoned types, in `t`'s time zone. The result has zero seconds.
    ///
    /// For a zoned `t`, the schedule runs on local wall-clock time and handles
    /// daylight-saving changes as Vixie cron does:
    /// - when the minute and hour fields are both fixed, like `30 2 * * *`, a time
    ///   skipped by a gap runs once, at the first instant after the gap, and a
    ///   time repeated by a fold runs once, at its first occurrence;
    /// - when the minute or hour field starts with `*`, like `*/5 * * * *`,
    ///   `30 * * * *` or `@hourly`, the schedule runs on real time: times skipped
    ///   by a gap don't run, and a repeated hour runs twice. From 02:55 in the
    ///   first pass, `*/5` runs next at 02:00 after the clocks went back.
    ///
    /// [`catch_up_gaps`](Self::catch_up_gaps) makes every schedule run skipped times
    /// after the gap.
    ///
    /// `None` means the schedule never fires (like `0 0 30 2 *`), or the next
    /// time doesn't fit in `T`.
    #[cfg(any(feature = "jiff", feature = "chrono"))]
    pub fn next_after<T: crate::CronTime>(&self, t: &T) -> Option<T> {
        T::next_for(self, t)
    }
}

impl fmt::Display for CronSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MONDAY: u8 = 1;
    const TUESDAY: u8 = 2;
    const WEDNESDAY: u8 = 3;

    #[test]
    fn matches_vixie() {
        // `0 0 */2 * MON`: the day-of-month field starts with `*`, so both must match.
        let odd_mondays = spec(
            "0 0 */2 * MON",
            1,
            1,
            0xAAAA_AAAA,
            0x1ffe,
            1 << MONDAY,
            true,
            false,
        );
        assert!(odd_mondays.matches(0, 0, 3, 6, MONDAY));
        assert!(!odd_mondays.matches(0, 0, 4, 6, MONDAY));
        assert!(!odd_mondays.matches(0, 0, 3, 6, TUESDAY));
        assert!(!odd_mondays.matches(1, 0, 3, 6, MONDAY));

        // `0 0 1 * MON`: both restricted, so either may match.
        let first_or_monday = spec(
            "0 0 1 * MON",
            1,
            1,
            1 << 1,
            0x1ffe,
            1 << MONDAY,
            false,
            false,
        );
        assert!(first_or_monday.matches(0, 0, 1, 6, WEDNESDAY));
        assert!(first_or_monday.matches(0, 0, 8, 6, MONDAY));
        assert!(!first_or_monday.matches(0, 0, 9, 6, TUESDAY));

        let every = spec(
            "* * * * *",
            (1 << 60) - 1,
            (1 << 24) - 1,
            !1,
            0x1ffe,
            0x7f,
            true,
            true,
        );
        assert!(every.matches(59, 23, 31, 12, 6));
        for (minute, hour, day, month, weekday) in [
            (60, 0, 1, 1, 0),
            (0, 24, 1, 1, 0),
            (0, 0, 0, 1, 0),
            (0, 0, 32, 1, 0),
            (0, 0, 1, 0, 0),
            (0, 0, 1, 13, 0),
            (0, 0, 1, 1, 7),
        ] {
            assert!(
                !every.matches(minute, hour, day, month, weekday),
                "{minute} {hour} {day} {month} {weekday}"
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn spec(
        source: &'static str,
        minutes: u64,
        hours: u32,
        days: u32,
        months: u16,
        weekdays: u8,
        dom_star: bool,
        dow_star: bool,
    ) -> CronSpec {
        let time_star = source.split(' ').take(2).any(|f| f.starts_with('*'));
        CronSpec::__new_unchecked(
            source, minutes, hours, days, months, weekdays, dom_star, dow_star, time_star,
        )
    }

    fn m(year: i32, month: u8, day: u8, hour: u8, minute: u8) -> Minute {
        Minute {
            year,
            month,
            day,
            hour,
            minute,
        }
    }

    const ALL_HOURS: u32 = (1 << 24) - 1;
    const ALL_DAYS: u32 = !1;
    const ALL_MONTHS: u16 = 0x1ffe;

    #[test]
    fn next_civil() {
        let quarter = spec(
            "*/15 * * * *",
            1 | 1 << 15 | 1 << 30 | 1 << 45,
            ALL_HOURS,
            ALL_DAYS,
            ALL_MONTHS,
            0x7f,
            true,
            true,
        );
        assert_eq!(
            quarter.next_civil_after(m(2026, 10, 30, 10, 7)),
            Some(m(2026, 10, 30, 10, 15))
        );
        assert_eq!(
            quarter.next_civil_after(m(2026, 10, 30, 10, 15)),
            Some(m(2026, 10, 30, 10, 30))
        );
        assert_eq!(
            quarter.next_civil_after(m(2026, 12, 31, 23, 45)),
            Some(m(2027, 1, 1, 0, 0))
        );

        let leap_day = spec("0 0 29 2 *", 1, 1, 1 << 29, 1 << 2, 0x7f, false, true);
        assert_eq!(
            leap_day.next_civil_after(m(2026, 3, 1, 0, 0)),
            Some(m(2028, 2, 29, 0, 0))
        );

        let never = spec("0 0 30 2 *", 1, 1, 1 << 30, 1 << 2, 0x7f, false, true);
        assert_eq!(never.next_civil_after(m(2026, 3, 1, 0, 0)), None);

        let last = spec("0 0 31 * *", 1, 1, 1 << 31, ALL_MONTHS, 0x7f, false, true);
        assert_eq!(
            last.next_civil_after(m(2026, 4, 15, 0, 0)),
            Some(m(2026, 5, 31, 0, 0))
        );

        // Both day fields restricted: the 13th or any Friday.
        let either = spec(
            "0 0 13 * 5",
            1,
            1,
            1 << 13,
            ALL_MONTHS,
            1 << 5,
            false,
            false,
        );
        assert_eq!(
            either.next_civil_after(m(2026, 10, 30, 0, 0)),
            Some(m(2026, 11, 6, 0, 0))
        );
        // `*/2` day of month: odd days that are Fridays.
        let odd_fridays = spec(
            "0 0 */2 * 5",
            1,
            1,
            0xAAAA_AAAA,
            ALL_MONTHS,
            1 << 5,
            true,
            false,
        );
        assert_eq!(
            odd_fridays.next_civil_after(m(2026, 10, 30, 0, 0)),
            Some(m(2026, 11, 13, 0, 0))
        );
    }
}
