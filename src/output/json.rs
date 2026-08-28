//! JSON rendering: an array of per-date objects, `{ "date", "zmanim": {..} }`.
//!
//! Zman key order follows the user's argument order. Missing zmanim render as
//! `null`. Values follow the resolved time-style (full-precision ISO by
//! default; human strings if `--time-style human` is forced).

use crate::compute::Grid;
use crate::output::value;
use crate::resolve::ResolvedSettings;
use serde_json::{Map, Value, json};

/// Renders the grid as a pretty-printed, newline-terminated JSON string.
pub fn render(grid: &Grid, settings: &ResolvedSettings) -> String {
    let style = value::resolve_style(settings.time_style, settings.format);

    let mut days = Vec::with_capacity(grid.dates.len());
    for (di, date) in grid.dates.iter().enumerate() {
        let mut zmanim = Map::new();
        for (ei, entry) in grid.entries.iter().enumerate() {
            let cell = match &grid.rows[di][ei] {
                None => Value::Null,
                Some(v) => {
                    Value::String(value::render(v, style, settings.precision, settings.round))
                }
            };
            zmanim.insert(entry.name.clone(), cell);
        }
        days.push(json!({
            "date": date.to_string(),
            "zmanim": Value::Object(zmanim),
        }));
    }

    let value = Value::Array(days);
    let mut out = serde_json::to_string_pretty(&value).expect("json serialization cannot fail");
    out.push('\n');
    out
}
