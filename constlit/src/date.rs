use core::fmt;

use crate::calendar::{day_of_year, days_in_month, weekday};

/// A calendar date in years 1-9999, usually made with [`date!`](crate::date).
///
/// Dates order chronologically.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    year: i16,
    month: u8,
    day: u8,
}

/// A day of the week.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Weekday {
    Sunday = 0,
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
}

impl Weekday {
    const ALL: [Weekday; 7] = [
        Weekday::Sunday,
        Weekday::Monday,
        Weekday::Tuesday,
        Weekday::Wednesday,
        Weekday::Thursday,
        Weekday::Friday,
        Weekday::Saturday,
    ];

    /// Monday = 1 … Sunday = 7, as in ISO 8601.
    pub const fn number_from_monday(self) -> u8 {
        match self {
            Weekday::Sunday => 7,
            day => day as u8,
        }
    }

    /// Sunday = 0 … Saturday = 6, as in cron and [`CronSpec::matches`](crate::CronSpec::matches).
    pub const fn num_days_from_sunday(self) -> u8 {
        self as u8
    }
}

impl Date {
    /// The date, or `None` outside years 1-9999 or for a day the month doesn't have.
    pub const fn new(year: i16, month: u8, day: u8) -> Option<Date> {
        if year < 1 || year > 9999 || month < 1 || month > 12 {
            return None;
        }
        if day < 1 || day > days_in_month(year as i32, month) {
            return None;
        }
        Some(Date { year, month, day })
    }

    #[doc(hidden)]
    pub const fn __new_unchecked(year: i16, month: u8, day: u8) -> Date {
        Date { year, month, day }
    }

    pub const fn year(&self) -> i16 {
        self.year
    }

    /// 1-12.
    pub const fn month(&self) -> u8 {
        self.month
    }

    /// 1-31.
    pub const fn day(&self) -> u8 {
        self.day
    }

    pub const fn weekday(&self) -> Weekday {
        Weekday::ALL[weekday(self.year as i32, self.month, self.day) as usize]
    }

    /// 1-366.
    pub const fn day_of_year(&self) -> u16 {
        day_of_year(self.year as i32, self.month, self.day)
    }

    /// Converts to a [`jiff::civil::Date`].
    #[cfg(feature = "jiff")]
    pub const fn to_jiff(&self) -> jiff::civil::Date {
        jiff::civil::Date::constant(self.year, self.month as i8, self.day as i8)
    }

    /// Converts to a [`chrono::NaiveDate`].
    #[cfg(feature = "chrono")]
    pub const fn to_chrono(&self) -> chrono::NaiveDate {
        match chrono::NaiveDate::from_ymd_opt(self.year as i32, self.month as u32, self.day as u32)
        {
            Some(date) => date,
            None => panic!("chrono supports years 1-9999"),
        }
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}
