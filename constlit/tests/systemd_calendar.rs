use constlit::systemd::{self, CalendarSpec};
use constlit::{date, time};

/// The oracle's expressions (oracle/systemd_fixtures.py ELAPSE_EXPRESSIONS).
macro_rules! specs {
    ($($lit:literal),* $(,)?) => {
        [$(($lit, systemd::calendar!($lit))),*]
    };
}

#[cfg_attr(not(any(feature = "jiff", feature = "chrono")), allow(dead_code))]
const SPECS: [(&str, CalendarSpec); 45] = specs![
    "minutely",
    "hourly",
    "daily",
    "weekly",
    "monthly",
    "yearly",
    "quarterly",
    "semiannually",
    "Mon *-*-* 00:00:00",
    "Mon..Wed,Fri 10:30",
    "*-*-01 06:00",
    "*-02~03",
    "*-*~07",
    "*:*:30",
    "*:0/15",
    "Wed 12:00 UTC",
    "*-1..3-1 00:00",
    "2026..2028-*-* 00:00",
    "mon 9:00",
    "2026-02-29",
    "8/2:00",
    "Fri *-*-1..7 18:00",
    "*-*-* *:*:*",
    "*-*-1,15 12:00",
    "00:00/2:00",
    "*-*-* 02:30",
    "*-*-* 02:00",
    "*-*-* 01:30",
    "*:00/30",
    "Sat,Sun 10:00",
    "*-*-31 00:00",
    "*-02-29 12:00",
    "2027-*-* 00:00",
    "*-*~01..03 23:00",
    "Mon..Fri 08..17:00/30",
    "*-*-* 03:00 UTC",
    "2029-02-29",
    "*-*-* 02:15:15",
    "*-*-* 01:45",
    "*-*-* 02:10",
    "Sun *-*-* 02:30",
    "*-03,10-* 02:30",
    "*-*~07/1",
    "Mon *-05~07/1",
    "*-*~10/3",
];

#[test]
fn calendar_matches() {
    const S: CalendarSpec = systemd::calendar!("Mon..Fri *-*-* 09:00");
    assert!(S.matches(date!("2026-11-02"), time!("09:00:00")));
    assert!(!S.matches(date!("2026-11-01"), time!("09:00:00")));
    assert!(!S.matches(date!("2026-11-02"), time!("09:00:01")));
    assert_eq!(S.to_string(), "Mon..Fri *-*-* 09:00:00");
    let end = systemd::calendar!("*-02~03");
    assert!(end.matches(date!("2026-02-26"), time!("00:00")));
    assert!(end.matches(date!("2028-02-27"), time!("00:00")));
    assert_eq!(
        systemd::calendar!("quarterly").to_string(),
        "*-01,04,07,10-01 00:00:00"
    );
    static ST: CalendarSpec = systemd::calendar!("daily");
    assert!(ST.matches(date!("2026-01-01"), time!("00:00")));
}

#[cfg(feature = "jiff")]
#[test]
fn calendar_elapse() {
    let fixture = include_str!("../../constlit-macros/src/testdata/systemd_elapse.tsv");
    let mut compared = 0;
    for line in fixture.lines().filter(|l| !l.starts_with('#')) {
        let f: Vec<&str> = line.split('\t').collect();
        let (zone, base, expr, expected) = (f[0], f[1].parse::<i64>().unwrap(), f[2], f[3]);
        let spec = SPECS.iter().find(|(lit, _)| *lit == expr).expect(expr).1;
        let tz = jiff::tz::TimeZone::get(zone).unwrap();
        let mut t = jiff::Timestamp::from_second(base).unwrap().to_zoned(tz);
        let mut ours = Vec::new();
        for _ in 0..5 {
            match spec.next_after(&t) {
                Some(next) => {
                    ours.push(next.timestamp().as_second().to_string());
                    t = next;
                }
                None => break,
            }
        }
        let ours = if ours.is_empty() {
            "never".to_string()
        } else {
            ours.join(",")
        };
        assert_eq!(ours, expected, "{expr} in {zone} after {base}");
        compared += 1;
    }
    assert!(compared >= 700, "only {compared} lines");
}

#[cfg(feature = "chrono")]
#[test]
fn calendar_chrono() {
    use chrono::{TimeZone, Utc};
    let fixture = include_str!("../../constlit-macros/src/testdata/systemd_elapse.tsv");
    for line in fixture.lines().filter(|l| !l.starts_with('#')).step_by(7) {
        let f: Vec<&str> = line.split('\t').collect();
        if f[0] != "UTC" && f[0] != "Europe/Stockholm" {
            continue;
        }
        let spec = SPECS.iter().find(|(lit, _)| *lit == f[2]).unwrap().1;
        let base = f[1].parse::<i64>().unwrap();
        let first = f[3].split(',').next().unwrap();
        let ours = if f[0] == "UTC" {
            spec.next_after(&Utc.timestamp_opt(base, 0).unwrap())
                .map(|t| t.timestamp())
        } else {
            let tz: chrono_tz::Tz = f[0].parse().unwrap();
            spec.next_after(&tz.timestamp_opt(base, 0).unwrap())
                .map(|t| t.timestamp())
        };
        let ours = ours.map_or("never".to_string(), |t| t.to_string());
        assert_eq!(ours, first, "{} in {} after {base}", f[2], f[0]);
    }
}
