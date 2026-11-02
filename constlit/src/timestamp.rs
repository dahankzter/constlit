use core::cmp::Ordering;
use core::fmt;
use core::hash::{Hash, Hasher};

use crate::calendar::civil_from_days;
use crate::time_of_day::write_fraction;
use crate::{Date, TimeOfDay};

/// Days from 1970-01-01 to 0001-01-01 and to 9999-12-31.
const FIRST_DAY: i64 = -719_162;
const LAST_DAY: i64 = 2_932_896;

/// An instant with the UTC offset it was written in, usually made with
/// [`ts!`](crate::ts).
///
/// Equality, ordering and hashing compare the instant only, so
/// `ts!("2026-10-30T14:00Z") == ts!("2026-10-30T15:00+01:00")`. The offset
/// decides [`date`](Self::date), [`time`](Self::time) and `Display`.
#[derive(Clone, Copy, Debug)]
pub struct Timestamp {
    unix_seconds: i64,
    nanosecond: u32,
    offset_seconds: i32,
}

impl Timestamp {
    /// The instant `unix_seconds + nanosecond / 10⁹` seconds after
    /// 1970-01-01T00:00Z, shown at `offset_seconds` east of UTC. `None` when
    /// `nanosecond` is 10⁹ or more, when the offset is a day or more, or when
    /// the local date falls outside years 1-9999.
    pub const fn new(unix_seconds: i64, nanosecond: u32, offset_seconds: i32) -> Option<Timestamp> {
        if nanosecond >= 1_000_000_000 || offset_seconds.unsigned_abs() >= 86_400 {
            return None;
        }
        let Some(local) = unix_seconds.checked_add(offset_seconds as i64) else {
            return None;
        };
        let day = local.div_euclid(86_400);
        if day < FIRST_DAY || day > LAST_DAY {
            return None;
        }
        Some(Timestamp {
            unix_seconds,
            nanosecond,
            offset_seconds,
        })
    }

    #[doc(hidden)]
    pub const fn __new_unchecked(
        unix_seconds: i64,
        nanosecond: u32,
        offset_seconds: i32,
    ) -> Timestamp {
        Timestamp {
            unix_seconds,
            nanosecond,
            offset_seconds,
        }
    }

    /// Whole seconds since 1970-01-01T00:00Z, rounded down: the instant is
    /// `unix_seconds() + nanosecond() / 10⁹`.
    pub const fn unix_seconds(&self) -> i64 {
        self.unix_seconds
    }

    /// 0-999,999,999.
    pub const fn nanosecond(&self) -> u32 {
        self.nanosecond
    }

    /// Seconds east of UTC.
    pub const fn offset_seconds(&self) -> i32 {
        self.offset_seconds
    }

    const fn local_seconds(&self) -> i64 {
        self.unix_seconds + self.offset_seconds as i64
    }

    /// The date at this offset.
    pub const fn date(&self) -> Date {
        let (year, month, day) = civil_from_days(self.local_seconds().div_euclid(86_400));
        Date::__new_unchecked(year as i16, month, day)
    }

    /// The time of day at this offset.
    pub const fn time(&self) -> TimeOfDay {
        let seconds = self.local_seconds().rem_euclid(86_400);
        TimeOfDay::__new_unchecked(
            (seconds / 3600) as u8,
            (seconds / 60 % 60) as u8,
            (seconds % 60) as u8,
            self.nanosecond,
        )
    }

    /// Converts to a [`jiff::Timestamp`]. `None` within about a day of the ends
    /// of years 1-9999, which jiff's range leaves out.
    #[cfg(feature = "jiff")]
    pub fn to_jiff(&self) -> Option<jiff::Timestamp> {
        jiff::Timestamp::new(self.unix_seconds, self.nanosecond as i32).ok()
    }

    /// Converts to a [`chrono::DateTime`] at this offset.
    #[cfg(feature = "chrono")]
    pub fn to_chrono(&self) -> chrono::DateTime<chrono::FixedOffset> {
        let offset = chrono::FixedOffset::east_opt(self.offset_seconds).expect("below a day");
        self.to_chrono_utc().with_timezone(&offset)
    }

    /// Converts to a [`chrono::DateTime`] in UTC.
    #[cfg(feature = "chrono")]
    pub fn to_chrono_utc(&self) -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::from_timestamp(self.unix_seconds, self.nanosecond)
            .expect("chrono covers years 1-9999")
    }

    /// Converts to a [`std::time::SystemTime`]. Panics where the platform's
    /// `SystemTime` can't reach the instant, such as before 1601 on Windows.
    #[cfg(feature = "std")]
    pub fn to_system_time(&self) -> std::time::SystemTime {
        use std::time::{Duration, UNIX_EPOCH};
        let fraction = Duration::from_nanos(self.nanosecond as u64);
        if self.unix_seconds >= 0 {
            UNIX_EPOCH + Duration::from_secs(self.unix_seconds as u64) + fraction
        } else {
            UNIX_EPOCH - Duration::from_secs(self.unix_seconds.unsigned_abs()) + fraction
        }
    }
}

impl PartialEq for Timestamp {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Timestamp {}

impl PartialOrd for Timestamp {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Timestamp {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.unix_seconds, self.nanosecond).cmp(&(other.unix_seconds, other.nanosecond))
    }
}

impl Hash for Timestamp {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.unix_seconds, self.nanosecond).hash(state);
    }
}

/// Writes `±HH:MM`, or `±HH:MM:SS` when the offset has seconds.
pub(crate) fn write_offset(f: &mut fmt::Formatter<'_>, offset_seconds: i32) -> fmt::Result {
    let sign = if offset_seconds < 0 { '-' } else { '+' };
    let magnitude = offset_seconds.unsigned_abs();
    write!(
        f,
        "{sign}{:02}:{:02}",
        magnitude / 3600,
        magnitude / 60 % 60
    )?;
    match magnitude % 60 {
        0 => Ok(()),
        seconds => write!(f, ":{seconds:02}"),
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let time = self.time();
        write!(
            f,
            "{}T{:02}:{:02}:{:02}",
            self.date(),
            time.hour(),
            time.minute(),
            time.second()
        )?;
        write_fraction(f, self.nanosecond)?;
        match self.offset_seconds {
            0 => f.write_str("Z"),
            offset => write_offset(f, offset),
        }
    }
}
