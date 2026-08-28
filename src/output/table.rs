//! Aligned text-table rendering.

use crate::compute::Grid;
use crate::output::value::{self, MISSING, RenderStyle};
use crate::resolve::ResolvedSettings;

/// Renders the grid as a text table.
///
/// A single date is shown as a vertical `name  value` list; multiple dates as
/// a `date | zman | zman ...` grid.
pub fn render(grid: &Grid, settings: &ResolvedSettings) -> String {
    let style = value::resolve_style(settings.time_style, settings.format);
    if grid.dates.len() == 1 {
        render_single(grid, settings, style)
    } else {
        render_multi(grid, settings, style)
    }
}

fn cell(
    grid: &Grid,
    settings: &ResolvedSettings,
    style: RenderStyle,
    di: usize,
    ei: usize,
) -> String {
    match &grid.rows[di][ei] {
        Some(v) => value::render(v, style, settings.precision, settings.round),
        None => MISSING.to_string(),
    }
}

fn render_single(grid: &Grid, settings: &ResolvedSettings, style: RenderStyle) -> String {
    let name_w = grid.entries.iter().map(|e| e.name.len()).max().unwrap_or(0);
    let mut out = String::new();
    for (ei, entry) in grid.entries.iter().enumerate() {
        let val = cell(grid, settings, style, 0, ei);
        out.push_str(&format!("{:<name_w$}  {}\n", entry.name, val));
    }
    out
}

fn render_multi(grid: &Grid, settings: &ResolvedSettings, style: RenderStyle) -> String {
    // Build all cells (as strings) first, then compute per-column widths.
    let mut header: Vec<String> = Vec::with_capacity(grid.entries.len() + 1);
    header.push("date".to_string());
    for e in &grid.entries {
        header.push(e.name.clone());
    }

    let mut body: Vec<Vec<String>> = Vec::with_capacity(grid.dates.len());
    for (di, date) in grid.dates.iter().enumerate() {
        let mut row = Vec::with_capacity(grid.entries.len() + 1);
        row.push(date.to_string());
        for ei in 0..grid.entries.len() {
            row.push(cell(grid, settings, style, di, ei));
        }
        body.push(row);
    }

    let cols = header.len();
    let mut widths = vec![0usize; cols];
    for (c, h) in header.iter().enumerate() {
        widths[c] = h.len();
    }
    for row in &body {
        for (c, cell) in row.iter().enumerate() {
            widths[c] = widths[c].max(cell.len());
        }
    }

    let mut out = String::new();
    push_row(&mut out, &header, &widths);
    for row in &body {
        push_row(&mut out, row, &widths);
    }
    out
}

fn push_row(out: &mut String, row: &[String], widths: &[usize]) {
    let line = row
        .iter()
        .enumerate()
        .map(|(c, cell)| format!("{:<w$}", cell, w = widths[c]))
        .collect::<Vec<_>>()
        .join("  ");
    // Trim trailing spaces from the last (padded) column.
    out.push_str(line.trim_end());
    out.push('\n');
}
