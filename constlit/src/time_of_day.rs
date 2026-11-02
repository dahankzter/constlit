use core::fmt;

/// A time of day on the 24-hour clock, with nanoseconds, usually made with
/// [`time!`](crate::time).
///
/// Times order chronologically. There is no leap second.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimeOfDay {
    hour: u8,
    minute: u8,
    second: u8,
    nanosecond: u32,
}

impl TimeOfDay {
    /// The time, or `None` when a field is out of range: hour 0-23, minute and
    /// second 0-59, nanosecond below 1,000,000,000.
    pub const fn new(hour: u8, minute: u8, second: u8, nanosecond: u32) -> Option<TimeOfDay> {
        if hour > 23 || minute > 59 || second > 59 || nanosecond >= 1_000_000_000 {
            return None;
        }
        Some(TimeOfDay {
            hour,
            minute,
            second,
            nanosecond,
        })
    }

    #[doc(hidden)]
    pub const fn __new_unchecked(hour: u8, minute: u8, second: u8, nanosecond: u32) -> TimeOfDay {
        TimeOfDay {
            hour,
            minute,
            second,
            nanosecond,
        }
    }

    pub const fn hour(&self) -> u8 {
        self.hour
    }

    pub const fn minute(&self) -> u8 {
        self.minute
    }

    pub const fn second(&self) -> u8 {
        self.second
    }

    pub const fn nanosecond(&self) -> u32 {
        self.nanosecond
    }

    /// Converts to a [`jiff::civil::Time`].
    #[cfg(feature = "jiff")]
    pub const fn to_jiff(&self) -> jiff::civil::Time {
        jiff::civil::Time::constant(
            self.hour as i8,
            self.minute as i8,
            self.second as i8,
            self.nanosecond as i32,
        )
    }

    /// Converts to a [`chrono::NaiveTime`].
    #[cfg(feature = "chrono")]
    pub const fn to_chrono(&self) -> chrono::NaiveTime {
        match chrono::NaiveTime::from_hms_nano_opt(
            self.hour as u32,
            self.minute as u32,
            self.second as u32,
            self.nanosecond,
        ) {
            Some(time) => time,
            None => panic!("validated by TimeOfDay"),
        }
    }
}

/// Writes `.` and the fraction of a second without trailing zeros, or nothing.
pub(crate) fn write_fraction(f: &mut fmt::Formatter<'_>, nanosecond: u32) -> fmt::Result {
    if nanosecond == 0 {
        return Ok(());
    }
    let mut digits = 9;
    let mut n = nanosecond;
    while n.is_multiple_of(10) {
        n /= 10;
        digits -= 1;
    }
    write!(f, ".{n:0digits$}")
}

impl fmt::Display for TimeOfDay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}:{:02}:{:02}", self.hour, self.minute, self.second)?;
        write_fraction(f, self.nanosecond)
    }
}
