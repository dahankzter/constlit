//! Proleptic Gregorian civil-date arithmetic, shared by the date and schedule types.

pub(crate) const fn is_leap(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

/// Days in `month` (1-12) of `year`.
pub(crate) const fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        2 if is_leap(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Day of the week, Sunday = 0, by Sakamoto's method.
pub(crate) const fn weekday(year: i32, month: u8, day: u8) -> u8 {
    const T: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = if month < 3 { year - 1 } else { year };
    let days = y + y.div_euclid(4) - y.div_euclid(100)
        + y.div_euclid(400)
        + T[month as usize - 1]
        + day as i32;
    days.rem_euclid(7) as u8
}

/// Day of the year, 1-366.
pub(crate) const fn day_of_year(year: i32, month: u8, day: u8) -> u16 {
    let mut days = day as u16;
    let mut m = 1;
    while m < month {
        days += days_in_month(year, m) as u16;
        m += 1;
    }
    days
}

/// The date `days` after 1970-01-01, by Howard Hinnant's `civil_from_days`.
pub(crate) const fn civil_from_days(days: i64) -> (i32, u8, u8) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + (month <= 2) as i64;
    (year as i32, month as u8, day as u8)
}

/// A wall-clock minute: a civil date and time without seconds or a time zone.
#[cfg_attr(not(any(feature = "jiff", feature = "chrono")), allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Minute {
    pub(crate) year: i32,
    pub(crate) month: u8,
    pub(crate) day: u8,
    pub(crate) hour: u8,
    pub(crate) minute: u8,
}

#[cfg_attr(not(any(feature = "jiff", feature = "chrono")), allow(dead_code))]
impl Minute {
    pub(crate) fn next(self) -> Minute {
        if self.minute < 59 {
            Minute {
                minute: self.minute + 1,
                ..self
            }
        } else {
            self.next_hour()
        }
    }

    pub(crate) fn prev(self) -> Minute {
        if self.minute > 0 {
            return Minute {
                minute: self.minute - 1,
                ..self
            };
        }
        let (year, month, day, hour) = match (self.hour, self.day, self.month) {
            (1.., ..) => (self.year, self.month, self.day, self.hour - 1),
            (0, 2.., _) => (self.year, self.month, self.day - 1, 23),
            (0, _, 2..) => {
                let month = self.month - 1;
                (self.year, month, days_in_month(self.year, month), 23)
            }
            _ => (self.year - 1, 12, 31, 23),
        };
        Minute {
            year,
            month,
            day,
            hour,
            minute: 59,
        }
    }

    /// The start of the next hour.
    pub(crate) fn next_hour(self) -> Minute {
        if self.hour < 23 {
            Minute {
                hour: self.hour + 1,
                minute: 0,
                ..self
            }
        } else {
            self.next_day()
        }
    }

    /// Midnight at the start of the next day.
    pub(crate) fn next_day(self) -> Minute {
        if self.day < days_in_month(self.year, self.month) {
            Minute {
                day: self.day + 1,
                hour: 0,
                minute: 0,
                ..self
            }
        } else {
            self.next_month()
        }
    }

    /// Midnight on the 1st of the next month.
    pub(crate) fn next_month(self) -> Minute {
        let (year, month) = if self.month < 12 {
            (self.year, self.month + 1)
        } else {
            (self.year + 1, 1)
        };
        Minute {
            year,
            month,
            day: 1,
            hour: 0,
            minute: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calendar_basics() {
        assert!(is_leap(2000));
        assert!(!is_leap(1900));
        assert!(is_leap(2024));
        assert!(!is_leap(2026));
        assert_eq!(days_in_month(2026, 2), 28);
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(2026, 4), 30);
        assert_eq!(days_in_month(2026, 12), 31);
        assert_eq!(weekday(2026, 10, 30), 5);
        assert_eq!(weekday(2000, 1, 1), 6);
        assert_eq!(weekday(1970, 1, 1), 4);
        assert_eq!(weekday(-1, 12, 31), 5);
    }

    #[test]
    fn calendar_day_of_year() {
        assert_eq!(day_of_year(2026, 1, 1), 1);
        assert_eq!(day_of_year(2026, 12, 31), 365);
        assert_eq!(day_of_year(2024, 12, 31), 366);
        assert_eq!(day_of_year(2024, 3, 1), 61);
        assert_eq!(day_of_year(2026, 10, 30), 303);
    }

    fn splitmix(state: &mut u64) -> u64 {
        *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = *state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    #[test]
    fn civil_from_days_oracle() {
        // 0001-01-01 and 9999-12-31 as days since 1970-01-01.
        const FIRST: i64 = -719_162;
        const LAST: i64 = 2_932_896;
        let mut state = 21u64;
        let random =
            (0..20_000).map(|_| FIRST + (splitmix(&mut state) % (LAST - FIRST + 1) as u64) as i64);
        for d in random.chain([FIRST, -1, 0, 1, LAST]) {
            let date = jiff::civil::date(1970, 1, 1)
                .checked_add(jiff::Span::new().days(d))
                .unwrap();
            assert_eq!(
                civil_from_days(d),
                (i32::from(date.year()), date.month() as u8, date.day() as u8),
                "day {d}"
            );
        }
    }

    #[test]
    fn minute_next() {
        let m = |year, month, day, hour, minute| Minute {
            year,
            month,
            day,
            hour,
            minute,
        };
        assert_eq!(m(2026, 10, 30, 10, 7).next(), m(2026, 10, 30, 10, 8));
        assert_eq!(m(2026, 10, 30, 10, 59).next(), m(2026, 10, 30, 11, 0));
        assert_eq!(m(2026, 10, 31, 23, 59).next(), m(2026, 11, 1, 0, 0));
        assert_eq!(m(2024, 2, 28, 23, 59).next(), m(2024, 2, 29, 0, 0));
        assert_eq!(m(2026, 12, 31, 23, 59).next(), m(2027, 1, 1, 0, 0));
    }

    #[test]
    fn minute_prev() {
        let m = |year, month, day, hour, minute| Minute {
            year,
            month,
            day,
            hour,
            minute,
        };
        for (a, b) in [
            (m(2026, 10, 30, 10, 7), m(2026, 10, 30, 10, 8)),
            (m(2026, 10, 30, 10, 59), m(2026, 10, 30, 11, 0)),
            (m(2026, 10, 31, 23, 59), m(2026, 11, 1, 0, 0)),
            (m(2024, 2, 29, 23, 59), m(2024, 3, 1, 0, 0)),
            (m(2026, 2, 28, 23, 59), m(2026, 3, 1, 0, 0)),
            (m(2026, 12, 31, 23, 59), m(2027, 1, 1, 0, 0)),
        ] {
            assert_eq!(b.prev(), a, "{b:?}");
        }
    }
}
