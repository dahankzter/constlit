#!/usr/bin/env python3
"""Writes the systemd fixtures constlit-macros' tests compare against, from
systemd-analyze itself. Each input runs in its own process: systemd-analyze
stops at the first failure, and omits "Original form" for normalized input.

Usage, from this directory:
    ./systemd_fixtures.py timespan > ../src/testdata/systemd_timespan.tsv
    ./systemd_fixtures.py calendar > ../src/testdata/systemd_calendar.tsv
    ./systemd_fixtures.py elapse > ../src/testdata/systemd_elapse.tsv

Inputs never contain tabs or newlines, so each line splits on tabs.
"""
import datetime
import random
import subprocess
import sys


def header():
    version = subprocess.run(["systemd-analyze", "--version"], capture_output=True, text=True).stdout
    print(f"# {version.splitlines()[0]}")


def timespans():
    units = ["us", "usec", "µs", "μs", "ms", "msec", "s", "sec", "second", "seconds", "m", "min",
             "minute", "minutes", "h", "hr", "hour", "hours", "d", "day", "days", "w", "week",
             "weeks", "M", "month", "months", "y", "year", "years"]
    fixed = ["", "5min 30s", "1h 30min", "2h30m", "1w2d", "1.5 hours", "1.5h", "500ms", "0",
             "1h,30min", "30", "1M", "1.0000001s", "infinity", " 5s", "5s ", "5 s", "1 month",
             "1y", "2 weeks", "1us", "0.5us", "5 parsecs", "1h 30", "-5s", "1.5.5s", "5mo",
             "5mins", "999999999999y", "1e3s", "1_000s", "+5s", ".5s", "5.s"]
    fixed += [f"1{u}" for u in units] + [f"2 {u}" for u in units]
    r = random.Random(9)
    numbers = ["0", "1", "2", "1.5", "0.25", "10", "007", "60", "3.000001", "1.1234567"]
    generated = []
    for _ in range(2000):
        parts = []
        for _ in range(r.randint(1, 3)):
            number = r.choice(numbers)
            unit = r.choice(units + ["", "x", "secs"])
            parts.append(number + r.choice(["", "", " "]) + unit)
        generated.append(r.choice(["", " ", ""]).join(parts))
    for s in fixed + generated:
        run = subprocess.run(["systemd-analyze", "timespan", s], capture_output=True, text=True)
        micros = [line.split(":", 1)[1].strip() for line in run.stdout.splitlines()
                  if line.strip().startswith("μs:")]
        if run.returncode == 0 and micros:
            print(f"{s}\tok\t{micros[0]}")
        else:
            print(f"{s}\terr")


CALENDAR_FIXED = [
    "minutely", "hourly", "daily", "weekly", "monthly", "yearly", "annually", "quarterly",
    "semiannually", "Mon *-*-* 00:00:00", "Mon..Wed,Fri 10:30", "*-*-01 06:00", "*-02~03",
    "*-*~07/1", "*:*:30", "*:0/15", "Wed 12:00 UTC", "*-1..3-1 00:00", "2026..2028-*-* 00:00",
    "mon 9:00", "2026-02-29", "8/2:00", "Fri *-*-1..7 18:00", "*-*-* *:*:*", "*-*-1,15 12:00",
    "00:00/2:00", "*-*-15,01 00:00", "*-*-03,01,02 00:00", "*-*-1..3,5 00:00", "*:5/10",
    "*:05..20/5", "Mon,Tue 00:00", "Mon,Tue,Wed 00:00", "Mon..Wed,Fri..Sun 00:00",
    "Sun,Mon 00:00", "Mon-Wed 00:00", "Monday 00:00", "Tue,Mon 00:00", "Mon,Mon 00:00",
    "*-*-* 00:00:00/30", "2026-1-1", "*-*-1/2", "2026/2-01-01", "*-1/3-1", "12:00:00 UTC",
    "12:00 utc", "*-*-* *:*", "*:*", "00/2:00", "*-*~1", "*-*~1,2", "*-*~1..3", "1..12:00",
    "*-*-01..31/7 00:00", "2026,2027-01-01", "*-*-* 1,2,3:00", "*-*-* 8..17:00",
    "Mon..Fri 8..17:0/30", "*-01-01", "01-01", "1-1", "Sat,Sun", "Fri",
    # Errors and exclusions.
    "", "Sat..Mon", "25:00", "*-*-* 25:00", "12:00:00.5", "Wed 12:00 Europe/Stockholm",
    "@1700000000", "today", "1,15 12:00", "*-13-01", "*-*-32", "*:60", "*-*-~01", "26-01-01",
    "1969-01-01", "2200-01-01", "*/2:00", "*-*-*/2", " daily", "daily ",
    "*:*:10..20", "12:00:00..30", "Mon..Sun", "monday-sun", "Mon..sat,Tue..Sun",
    "Mon..Sun 12:00 UTC", "*-*~07/1", "Mon *-05~07/1", "*-*~10/3", "*-*~03/3", "Daily",
    "DAILY", "12:00  UTC", "12:00 GMT", "2026-02", "*,5:00", "-01-01", "Mon  12:00",
    "*-*-01/4294967295", "1..5/2147483648:00",
] + [f"*-*~{n:02}/{k}" for n in range(1, 29) for k in (1, 2, 3, 7, 14, 27)]


def calendar_component(r, low, high, width):
    pick = r.random()
    n = lambda: str(r.randint(low, high)).zfill(r.choice([1, width]))
    if pick < 0.3:
        return "*"
    if pick < 0.55:
        return n()
    if pick < 0.7:
        return ",".join(n() for _ in range(r.randint(2, 3)))
    if pick < 0.82:
        return f"{n()}..{n()}"
    if pick < 0.94:
        return f"{n()}/{r.randint(1, 7)}"
    return r.choice(["*/2", f"{high + 1}", "0", f"{n()}..{n()}/{r.randint(1, 5)}"])


def calendars():
    r = random.Random(10)
    weekdays = ["", "", "", "Mon", "mon", "Mon,Wed", "Mon..Fri", "Sat-Sun", "Fri,Mon",
                "Sun..Tue", "Monday", "Tue,Wed,Thu", "Sat,Sun"]
    generated = []
    for _ in range(2000):
        parts = []
        wd = r.choice(weekdays)
        if wd:
            parts.append(wd)
        if r.random() < 0.6:
            year = r.choice(["*", "*", "2026", "2026..2028", "2026/2", "2027,2026"])
            month = calendar_component(r, 1, 12, 2)
            day = calendar_component(r, 1, 31, 2)
            sep = "~" if r.random() < 0.1 else "-"
            parts.append(f"{year}-{month}{sep}{day}" if r.random() < 0.7 else f"{month}{sep}{day}")
        if r.random() < 0.8 or not parts:
            hour = calendar_component(r, 0, 23, 2)
            minute = calendar_component(r, 0, 59, 2)
            time = f"{hour}:{minute}"
            if r.random() < 0.4:
                time += ":" + calendar_component(r, 0, 59, 2)
            parts.append(time)
        if r.random() < 0.1:
            parts.append("UTC")
        generated.append(" ".join(parts))
    for s in CALENDAR_FIXED + generated:
        run = subprocess.run(
            ["systemd-analyze", "calendar", "--iterations=1", "--base-time=2026-11-02 12:00:00 UTC", s],
            capture_output=True, text=True, env={"TZ": "UTC", "PATH": "/usr/bin:/bin"})
        normalized = [line.split(":", 1)[1].strip() for line in run.stdout.splitlines()
                      if line.strip().startswith("Normalized form:")]
        if run.returncode == 0 and normalized:
            print(f"{s}\tok\t{normalized[0]}")
        else:
            print(f"{s}\terr")


# Expressions for next-elapse checks; constlit/tests/systemd_calendar.rs lists the same.
ELAPSE_EXPRESSIONS = [
    "minutely", "hourly", "daily", "weekly", "monthly", "yearly", "quarterly", "semiannually",
    "Mon *-*-* 00:00:00", "Mon..Wed,Fri 10:30", "*-*-01 06:00", "*-02~03", "*-*~07",
    "*:*:30", "*:0/15", "Wed 12:00 UTC", "*-1..3-1 00:00", "2026..2028-*-* 00:00", "mon 9:00",
    "2026-02-29", "8/2:00", "Fri *-*-1..7 18:00", "*-*-* *:*:*", "*-*-1,15 12:00",
    "00:00/2:00", "*-*-* 02:30", "*-*-* 02:00", "*-*-* 01:30", "*:00/30", "Sat,Sun 10:00",
    "*-*-31 00:00", "*-02-29 12:00", "2027-*-* 00:00", "*-*~01..03 23:00",
    "Mon..Fri 08..17:00/30", "*-*-* 03:00 UTC", "2029-02-29", "*-*-* 02:15:15",
    "*-*-* 01:45", "*-*-* 02:10", "Sun *-*-* 02:30", "*-03,10-* 02:30",
    "*-*~07/1", "Mon *-05~07/1", "*-*~10/3",
]

ELAPSE_BASES = {
    "UTC": [],
    "Europe/Stockholm": ["2026-03-28 12:00:00", "2026-10-24 12:00:00"],
    "America/New_York": ["2026-03-07 12:00:00", "2026-10-31 12:00:00"],
    "Australia/Lord_Howe": ["2026-04-04 12:00:00", "2026-10-03 12:00:00"],
}
COMMON_BASES = ["2026-11-02 12:00:00", "2026-01-15 03:17:00", "2026-06-30 23:59:30"]


def unix(text):
    # "Mon 2026-11-02 12:01:00 UTC"
    stamp = " ".join(text.split()[1:3])
    moment = datetime.datetime.strptime(stamp, "%Y-%m-%d %H:%M:%S")
    return int(moment.replace(tzinfo=datetime.timezone.utc).timestamp())


def elapses():
    for zone, extra in ELAPSE_BASES.items():
        for base in COMMON_BASES + extra:
            base_unix = unix(f"x {base} UTC")
            for expr in ELAPSE_EXPRESSIONS:
                run = subprocess.run(
                    ["systemd-analyze", "calendar", "--iterations=5", f"--base-time={base} UTC", expr],
                    capture_output=True, text=True, env={"TZ": zone, "PATH": "/usr/bin:/bin"})
                lines = [line.strip() for line in run.stdout.splitlines()]
                if zone == "UTC":
                    times = [l.split(":", 1)[1].strip() for l in lines
                             if l.startswith(("Next elapse:", "Iteration #"))]
                else:
                    times = [l.split(":", 1)[1].strip() for l in lines if l.startswith("(in UTC):")]
                times = [t for t in times if t != "never"]
                elapsed = ",".join(str(unix(t)) for t in times) or "never"
                print(f"{zone}\t{base_unix}\t{expr}\t{elapsed}")


def main():
    header()
    {"timespan": timespans, "calendar": calendars, "elapse": elapses}[sys.argv[1]]()


main()
