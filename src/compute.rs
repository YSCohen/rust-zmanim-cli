//! Runs the actual zman computation over the resolved dates × zmanim.

use crate::resolve::ResolvedSettings;
use jiff::civil::Date;
use rust_zmanim::complex_zmanim_calendar::{ZmanEntry, ZmanValue};
use rust_zmanim::prelude::ComplexZmanimCalendar;

/// A computed grid: one row per date, one column per requested zman.
#[derive(Debug)]
pub struct Grid {
    /// The dates, in order (row order).
    pub dates: Vec<Date>,
    /// The zman entries, in order (column order).
    pub entries: Vec<&'static ZmanEntry>,
    /// `rows[date_index][entry_index]`. `None` when the zman does not occur.
    pub rows: Vec<Vec<Option<ZmanValue>>>,
}

/// Computes every requested zman for every date, reusing a single calendar.
pub fn compute(settings: &ResolvedSettings) -> Grid {
    debug_assert!(!settings.dates.is_empty());
    let mut czc = ComplexZmanimCalendar::new(
        settings.geo.clone(),
        settings.dates[0],
        settings.use_elevation,
    );

    let rows = settings
        .dates
        .iter()
        .map(|&date| {
            czc.set_date(date);
            settings
                .entries
                .iter()
                .map(|entry| (entry.compute)(&czc))
                .collect()
        })
        .collect();

    Grid {
        dates: settings.dates.clone(),
        entries: settings.entries.clone(),
        rows,
    }
}
