//! Output rendering: dispatches to table / csv / json per the resolved format.

mod csv;
mod json;
mod table;
pub mod value;

use crate::cli::Format;
use crate::compute::Grid;
use crate::resolve::ResolvedSettings;

/// Renders the computed grid to a string in the resolved output format.
pub fn render(grid: &Grid, settings: &ResolvedSettings) -> String {
    match settings.format {
        Format::Table => table::render(grid, settings),
        Format::Csv => csv::render(grid, settings),
        Format::Json => json::render(grid, settings),
    }
}
