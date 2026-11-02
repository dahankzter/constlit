use core::cmp::Ordering;
use core::fmt;
use core::hash::{Hash, Hasher};

use crate::timestamp::write_offset;
use crate::{Date, TimeOfDay, Timestamp};

/// An instant in a named IANA time zone, made with [`ts!`](crate::ts).
///
/// The offset was resolved from the zone at compile time. Equality, ordering and
/// hashing compare the instant only, like [`Timestamp`].
#[derive(Clone, Copy, Debug)]
pub struct ZonedTimestamp {
    timestamp: Timestamp,
    zone: &'static str,
}

impl ZonedTimestamp {
    #[doc(hidden)]
    pub const fn __new_unchecked(timestamp: Timestamp, zone: &'static str) -> ZonedTimestamp {
        ZonedTimestamp { timestamp, zone }
    }

    /// The instant with the zone's offset at that instant.
    pub const fn timestamp(&self) -> Timestamp {
        self.timestamp
    }

    /// The zone name as written, like `"Europe/Stockholm"`.
    pub const fn zone(&self) -> &'static str {
        self.zone
    }

    pub const fn unix_seconds(&self) -> i64 {
        self.timestamp.unix_seconds()
    }

    pub const fn nanosecond(&self) -> u32 {
        self.timestamp.nanosecond()
    }

    pub const fn offset_seconds(&self) -> i32 {
        self.timestamp.offset_seconds()
    }

    /// The date in the zone.
    pub const fn date(&self) -> Date {
        self.timestamp.date()
    }

    /// The time of day in the zone.
    pub const fn time(&self) -> TimeOfDay {
        self.timestamp.time()
    }

    /// Converts to a [`jiff::Zoned`], looking the zone up in jiff's time-zone
    /// database at run time. That needs a database feature in your jiff, such as
    /// `tzdb-bundle-always` or the default `tz-system`. If that database's rules
    /// differ from the ones bundled at compile time, the offset can differ too.
    #[cfg(feature = "jiff")]
    pub fn to_jiff_zoned(&self) -> Result<jiff::Zoned, jiff::Error> {
        let zone = jiff::tz::TimeZone::get(self.zone)?;
        let timestamp = jiff::Timestamp::new(self.unix_seconds(), self.nanosecond() as i32)?;
        Ok(timestamp.to_zoned(zone))
    }
}

impl PartialEq for ZonedTimestamp {
    fn eq(&self, other: &Self) -> bool {
        self.timestamp == other.timestamp
    }
}

impl Eq for ZonedTimestamp {}

impl PartialOrd for ZonedTimestamp {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ZonedTimestamp {
    fn cmp(&self, other: &Self) -> Ordering {
        self.timestamp.cmp(&other.timestamp)
    }
}

impl Hash for ZonedTimestamp {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.timestamp.hash(state);
    }
}

/// Like `2026-10-30T14:00:00+01:00 Europe/Stockholm`, which `ts!` accepts back.
impl fmt::Display for ZonedTimestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = self.timestamp;
        let time = s.time();
        write!(
            f,
            "{}T{:02}:{:02}:{:02}",
            s.date(),
            time.hour(),
            time.minute(),
            time.second()
        )?;
        crate::time_of_day::write_fraction(f, s.nanosecond())?;
        write_offset(f, s.offset_seconds())?;
        write!(f, " {}", self.zone)
    }
}
