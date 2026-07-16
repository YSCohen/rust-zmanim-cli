//! CSV rendering (always dates-as-rows with a header, for a stable machine
//! format). Rendered values and zman names never contain `,` or `"`, so no
//! quoting layer is needed; missing values are empty fields.

use crate::compute::Grid;
use crate::output::value;
use crate::resolve::ResolvedSettings;

/// Renders the grid as CSV.
pub fn render(grid: &Grid, settings: &ResolvedSettings) -> String {
    let style = value::resolve_style(settings.time_style, settings.format);

    let mut out = String::new();

    // Header.
    out.push_str("date");
    for e in &grid.entries {
        out.push(',');
        out.push_str(e.name);
    }
    out.push('\n');

    // Rows.
    for (di, date) in grid.dates.iter().enumerate() {
        out.push_str(&date.to_string());
        for ei in 0..grid.entries.len() {
            out.push(',');
            if let Some(v) = &grid.rows[di][ei] {
                out.push_str(&value::render(v, style, settings.precision, settings.round));
            }
        }
        out.push('\n');
    }

    out
}
