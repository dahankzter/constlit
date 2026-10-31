//! The date-time types `CronSpec::next_after` works with.

use crate::CronSpec;
use crate::calendar::Minute;

/// A date-time type [`CronSpec::next_after`] works with.
///
/// Implemented, behind the matching features, for `jiff::civil::DateTime`,
/// `jiff::Zoned`, `chrono::NaiveDateTime` and `chrono::DateTime<Tz>`.
pub trait CronTime: Sized + sealed::Sealed {}

mod sealed {
    use crate::CronSpec;

    pub trait Sealed: Sized {
        fn next_for(spec: &CronSpec, t: &Self) -> Option<Self>;
    }
}

impl<T: sealed::Sealed> CronTime for T {}

/// Finds the next match for a zoned `t`, whose local time is `local`.
///
/// `resolve` turns a wall-clock minute into an instant: `Ok(Some(..))` for a single
/// or the earlier of two instants, `Ok(None)` for a minute in a gap, `Err(())`
/// when the minute doesn't fit the type. `after` says whether an instant is
/// strictly after `t`.
fn next_zoned<Z>(
    spec: &CronSpec,
    local: Minute,
    resolve: impl Fn(Minute) -> Result<Option<Z>, ()>,
    after: impl Fn(&Z) -> bool,
) -> Option<Z> {
    let mut cursor = local;
    loop {
        let candidate = spec.next_civil_after(cursor)?;
        let mut m = candidate;
        let instant = loop {
            match resolve(m).ok()? {
                Some(instant) => break instant,
                None => m = m.next(),
            }
        };
        if after(&instant) {
            return Some(instant);
        }
        cursor = candidate;
    }
}

#[cfg(feature = "jiff")]
mod jiff_impl {
    use super::{Minute, next_zoned, sealed::Sealed};
    use crate::CronSpec;
    use jiff::Zoned;
    use jiff::civil::DateTime;
    use jiff::tz::AmbiguousOffset;

    fn minute(t: &DateTime) -> Minute {
        Minute {
            year: t.year().into(),
            month: t.month() as u8,
            day: t.day() as u8,
            hour: t.hour() as u8,
            minute: t.minute() as u8,
        }
    }

    fn civil(m: Minute) -> Result<DateTime, ()> {
        let year = i16::try_from(m.year).map_err(|_| ())?;
        DateTime::new(
            year,
            m.month as i8,
            m.day as i8,
            m.hour as i8,
            m.minute as i8,
            0,
            0,
        )
        .map_err(|_| ())
    }

    impl Sealed for DateTime {
        fn next_for(spec: &CronSpec, t: &Self) -> Option<Self> {
            civil(spec.next_civil_after(minute(t))?).ok()
        }
    }

    impl Sealed for Zoned {
        fn next_for(spec: &CronSpec, t: &Self) -> Option<Self> {
            let tz = t.time_zone();
            let resolve = |m: Minute| {
                let ambiguous = tz.to_ambiguous_zoned(civil(m)?);
                if let AmbiguousOffset::Gap { .. } = ambiguous.offset() {
                    return Ok(None);
                }
                ambiguous.earlier().map(Some).map_err(|_| ())
            };
            next_zoned(spec, minute(&t.datetime()), resolve, |z: &Zoned| {
                z.timestamp() > t.timestamp()
            })
        }
    }
}

#[cfg(feature = "chrono")]
mod chrono_impl {
    use super::{Minute, next_zoned, sealed::Sealed};
    use crate::CronSpec;
    use chrono::{DateTime, Datelike, LocalResult, NaiveDate, NaiveDateTime, TimeZone, Timelike};

    fn minute(t: &NaiveDateTime) -> Minute {
        Minute {
            year: t.year(),
            month: t.month() as u8,
            day: t.day() as u8,
            hour: t.hour() as u8,
            minute: t.minute() as u8,
        }
    }

    fn naive(m: Minute) -> Result<NaiveDateTime, ()> {
        NaiveDate::from_ymd_opt(m.year, m.month.into(), m.day.into())
            .and_then(|d| d.and_hms_opt(m.hour.into(), m.minute.into(), 0))
            .ok_or(())
    }

    impl Sealed for NaiveDateTime {
        fn next_for(spec: &CronSpec, t: &Self) -> Option<Self> {
            naive(spec.next_civil_after(minute(t))?).ok()
        }
    }

    impl<Tz: TimeZone> Sealed for DateTime<Tz> {
        fn next_for(spec: &CronSpec, t: &Self) -> Option<Self> {
            let tz = t.timezone();
            let resolve = |m: Minute| match tz.from_local_datetime(&naive(m)?) {
                LocalResult::Single(instant) | LocalResult::Ambiguous(instant, _) => {
                    Ok(Some(instant))
                }
                LocalResult::None => Ok(None),
            };
            next_zoned(
                spec,
                minute(&t.naive_local()),
                resolve,
                |z: &DateTime<Tz>| z > t,
            )
        }
    }
}
