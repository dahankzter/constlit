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

/// How a wall-clock minute maps to instants in a time zone.
enum Local<Z> {
    /// Skipped by a gap.
    Gap,
    One(Z),
    /// Repeated by a fold: the earlier and the later instant.
    Two(Z, Z),
}

/// Finds the next match strictly after the zoned `t`, whose local time is `local`.
///
/// `resolve` maps a wall-clock minute to its instants, or fails when the minute
/// doesn't fit the type.
fn next_zoned<Z: Ord>(
    spec: &CronSpec,
    t: &Z,
    local: Minute,
    resolve: impl Fn(Minute) -> Result<Local<Z>, ()>,
) -> Option<Z> {
    let mut cursor = local;
    let next = loop {
        let candidate = spec.next_civil_after(cursor)?;
        let instant = match resolve(candidate).ok()? {
            Local::One(instant) => instant,
            // Wildcard schedules also run in the second pass of a fold.
            Local::Two(early, late) if early <= *t && spec.time_star => late,
            Local::Two(early, _) => early,
            // Vixie: wildcard schedules skip the minutes a gap jumps over.
            Local::Gap if spec.time_star && !spec.catch_up_gaps => {
                cursor = candidate;
                continue;
            }
            // Others run a skipped minute at the first instant after the gap.
            Local::Gap => {
                let mut m = candidate.next();
                loop {
                    match resolve(m).ok()? {
                        Local::Gap => m = m.next(),
                        Local::One(instant) | Local::Two(instant, _) => break instant,
                    }
                }
            }
        };
        if instant > *t {
            break instant;
        }
        cursor = candidate;
    };
    // From the first pass of a fold, a wildcard schedule's next run may be in the
    // second pass, at a wall-clock minute before `local`.
    if spec.time_star
        && let Ok(Local::Two(_, late)) = resolve(local)
        && late > *t
    {
        let mut start = local;
        while let Ok(Local::Two(..)) = resolve(start.prev()) {
            start = start.prev();
        }
        if let Some(m) = spec.next_civil_after(start.prev())
            && let Ok(Local::Two(_, late)) = resolve(m)
        {
            return Some(next.min(late));
        }
    }
    Some(next)
}

#[cfg(feature = "jiff")]
mod jiff_impl {
    use super::{Minute, next_zoned, sealed::Sealed};
    use crate::CronSpec;
    use jiff::Zoned;
    use jiff::civil::DateTime;
    use jiff::tz::AmbiguousOffset;

    use super::Local;

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
                Ok(match ambiguous.offset() {
                    AmbiguousOffset::Gap { .. } => Local::Gap,
                    AmbiguousOffset::Unambiguous { .. } => {
                        Local::One(ambiguous.compatible().map_err(|_| ())?)
                    }
                    AmbiguousOffset::Fold { .. } => Local::Two(
                        ambiguous.clone().earlier().map_err(|_| ())?,
                        ambiguous.later().map_err(|_| ())?,
                    ),
                })
            };
            next_zoned(spec, t, minute(&t.datetime()), resolve)
        }
    }
}

#[cfg(feature = "chrono")]
mod chrono_impl {
    use super::{Local, Minute, next_zoned, sealed::Sealed};
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
            let resolve = |m: Minute| {
                Ok(match tz.from_local_datetime(&naive(m)?) {
                    LocalResult::Single(instant) => Local::One(instant),
                    LocalResult::Ambiguous(early, late) => Local::Two(early, late),
                    LocalResult::None => Local::Gap,
                })
            };
            next_zoned(spec, t, minute(&t.naive_local()), resolve)
        }
    }
}
