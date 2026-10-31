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

/// A wall-clock minute: a civil date and time without seconds or a time zone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Minute {
    pub(crate) year: i32,
    pub(crate) month: u8,
    pub(crate) day: u8,
    pub(crate) hour: u8,
    pub(crate) minute: u8,
}

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
}
