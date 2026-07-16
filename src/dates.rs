//! Parsing the `--date` grammar and expanding it into a list of dates.
//!
//! Grammar: a single date `YYYY-MM-DD`, the keywords `today` / `tomorrow`, or
//! an inclusive range `START..END` where each endpoint is itself a single
//! token. `--days N` is an alternate way to spell a range starting at `--date`
//! (or today).

use anyhow::{Result, bail};
use jiff::{Zoned, civil::Date, tz::TimeZone};

/// Resolves the `--date` / `--days` arguments into a non-empty list of dates,
/// interpreting `today`/`tomorrow` in `tz`.
pub fn resolve_dates(
    date_arg: Option<&str>,
    days: Option<u32>,
    tz: &TimeZone,
) -> Result<Vec<Date>> {
    let today = Zoned::now().with_time_zone(tz.clone()).date();

    let is_range = date_arg.map(|s| s.contains("..")).unwrap_or(false);

    if is_range {
        if days.is_some() {
            bail!("--days cannot be combined with a date range");
        }
        let s = date_arg.expect("is_range implies date_arg is Some");
        let (start_s, end_s) = s.split_once("..").expect("contains '..'");
        let start = parse_single(start_s.trim(), today)?;
        let end = parse_single(end_s.trim(), today)?;
        if start > end {
            bail!("date range start ({start}) is after end ({end})");
        }
        return expand_inclusive(start, end);
    }

    if let Some(n) = days {
        if n < 1 {
            bail!("--days must be at least 1");
        }
        let start = match date_arg {
            Some(s) => parse_single(s.trim(), today)?,
            None => today,
        };
        return expand_count(start, n as usize);
    }

    match date_arg {
        Some(s) => Ok(vec![parse_single(s.trim(), today)?]),
        None => Ok(vec![today]),
    }
}

/// Parses a single date token: `today`, `tomorrow`, or an ISO `YYYY-MM-DD`.
fn parse_single(s: &str, today: Date) -> Result<Date> {
    match s {
        "today" => Ok(today),
        "tomorrow" => today
            .tomorrow()
            .map_err(|e| anyhow::anyhow!("could not compute tomorrow: {e}")),
        "" => bail!("empty date"),
        other => other.parse::<Date>().map_err(|_| {
            anyhow::anyhow!("invalid date '{other}' (expected YYYY-MM-DD, 'today', or 'tomorrow')")
        }),
    }
}

/// Expands an inclusive `[start, end]` range.
fn expand_inclusive(start: Date, end: Date) -> Result<Vec<Date>> {
    let mut out = Vec::new();
    let mut d = start;
    loop {
        out.push(d);
        if d == end {
            break;
        }
        d = d
            .tomorrow()
            .map_err(|e| anyhow::anyhow!("date overflow while expanding range: {e}"))?;
    }
    Ok(out)
}

/// Expands `count` consecutive days starting at `start`.
fn expand_count(start: Date, count: usize) -> Result<Vec<Date>> {
    let mut out = Vec::with_capacity(count);
    let mut d = start;
    for _ in 0..count {
        out.push(d);
        d = d
            .tomorrow()
            .map_err(|e| anyhow::anyhow!("date overflow while expanding --days: {e}"))?;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::date;

    fn utc() -> TimeZone {
        TimeZone::UTC
    }

    #[test]
    fn single_iso_date() {
        let d = resolve_dates(Some("2026-07-14"), None, &utc()).unwrap();
        assert_eq!(d, vec![date(2026, 7, 14)]);
    }

    #[test]
    fn inclusive_range() {
        let d = resolve_dates(Some("2026-07-14..2026-07-16"), None, &utc()).unwrap();
        assert_eq!(
            d,
            vec![date(2026, 7, 14), date(2026, 7, 15), date(2026, 7, 16)]
        );
    }

    #[test]
    fn range_endpoint_keyword() {
        // today..today collapses to a single day
        let d = resolve_dates(Some("today..today"), None, &utc()).unwrap();
        assert_eq!(d.len(), 1);
    }

    #[test]
    fn reversed_range_errors() {
        let e = resolve_dates(Some("2026-07-16..2026-07-14"), None, &utc()).unwrap_err();
        assert!(e.to_string().contains("after end"));
    }

    #[test]
    fn days_from_date() {
        let d = resolve_dates(Some("2026-07-14"), Some(3), &utc()).unwrap();
        assert_eq!(
            d,
            vec![date(2026, 7, 14), date(2026, 7, 15), date(2026, 7, 16)]
        );
    }

    #[test]
    fn days_from_today() {
        let d = resolve_dates(None, Some(5), &utc()).unwrap();
        assert_eq!(d.len(), 5);
    }

    #[test]
    fn days_with_range_errors() {
        let e = resolve_dates(Some("2026-07-14..2026-07-16"), Some(3), &utc()).unwrap_err();
        assert!(e.to_string().contains("cannot be combined"));
    }

    #[test]
    fn zero_days_errors() {
        assert!(resolve_dates(None, Some(0), &utc()).is_err());
    }

    #[test]
    fn no_args_is_today() {
        let d = resolve_dates(None, None, &utc()).unwrap();
        assert_eq!(d.len(), 1);
    }

    #[test]
    fn invalid_date_errors() {
        assert!(resolve_dates(Some("not-a-date"), None, &utc()).is_err());
    }
}
