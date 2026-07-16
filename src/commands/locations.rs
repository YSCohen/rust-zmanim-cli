//! `zmanim locations` - list and manage saved locations.
//!
//! Mutations go through `toml_edit` so that user comments and formatting in the
//! config file are preserved.

use crate::cli::{
    LocationAddArgs, LocationNameArg, LocationsArgs, LocationsCommand, UseElevationArg,
};
use crate::config;
use anyhow::{Context, Result, bail};
use jiff::tz::TimeZone;
use rust_zmanim::prelude::GeoLocation;
use std::path::{Path, PathBuf};

/// Entry point for the `locations` subcommand.
pub fn run(args: &LocationsArgs, config_override: Option<PathBuf>) -> Result<()> {
    let path = config::config_path(config_override)?;
    match args.command.as_ref().unwrap_or(&LocationsCommand::List) {
        LocationsCommand::List => list(&path),
        LocationsCommand::Add(a) => add(&path, a),
        LocationsCommand::Remove(a) => remove(&path, a),
        LocationsCommand::SetDefault(a) => set_default(&path, a),
    }
}

fn list(path: &Path) -> Result<()> {
    let config = config::load(path)?;
    if config.locations.is_empty() {
        println!("no saved locations (add one with 'zmanim locations add')");
        return Ok(());
    }

    // Build rows.
    let mut rows: Vec<[String; 7]> = Vec::new();
    let header = [
        "".to_string(),
        "name".to_string(),
        "lat".to_string(),
        "lon".to_string(),
        "elev".to_string(),
        "tz".to_string(),
        "use-elevation".to_string(),
    ];
    rows.push(header);
    for (name, e) in &config.locations {
        let marker = if config.default_location.as_deref() == Some(name.as_str()) {
            "*".to_string()
        } else {
            String::new()
        };
        // Blank when unset: it *defaults* to 0 at compute time, but an
        // explicit 0 and an unset elevation are different config states.
        let elev = e.elevation.map(|v| format!("{v}")).unwrap_or_default();
        let ue = e
            .use_elevation
            .map(use_elevation_str)
            .unwrap_or("")
            .to_string();
        rows.push([
            marker,
            name.clone(),
            format!("{}", e.latitude),
            format!("{}", e.longitude),
            elev,
            e.timezone.clone(),
            ue,
        ]);
    }

    let mut widths = [0usize; 7];
    for row in &rows {
        for (i, cell) in row.iter().enumerate() {
            widths[i] = widths[i].max(cell.len());
        }
    }
    for row in &rows {
        let line = row
            .iter()
            .enumerate()
            .map(|(i, c)| format!("{:<w$}", c, w = widths[i]))
            .collect::<Vec<_>>()
            .join("  ");
        println!("{}", line.trim_end());
    }
    Ok(())
}

fn add(path: &Path, a: &LocationAddArgs) -> Result<()> {
    // Validate before writing anything.
    let tz = TimeZone::get(&a.tz)
        .map_err(|_| anyhow::anyhow!("unknown timezone '{}' (expected an IANA name)", a.tz))?;
    GeoLocation::new(a.lat, a.lon, a.elevation.unwrap_or(0.0), tz)
        .map_err(|e| anyhow::anyhow!("invalid location: {e}"))?;

    // Was this an update, and were there any locations / a default before?
    let existing = config::load(path).unwrap_or_default();
    let is_update = existing.locations.contains_key(&a.name);
    let should_default =
        a.default || (existing.locations.is_empty() && existing.default_location.is_none());

    let mut doc = read_doc(path)?;

    ensure_table(&mut doc, "locations");
    let locations = doc["locations"]
        .as_table_mut()
        .context("`locations` in config is not a table")?;

    let mut tbl = toml_edit::Table::new();
    tbl["latitude"] = toml_edit::value(a.lat);
    tbl["longitude"] = toml_edit::value(a.lon);
    if let Some(e) = a.elevation {
        tbl["elevation"] = toml_edit::value(e);
    }
    tbl["timezone"] = toml_edit::value(a.tz.clone());
    if let Some(ue) = a.use_elevation {
        tbl["use_elevation"] = toml_edit::value(use_elevation_str(ue));
    }
    locations.insert(&a.name, toml_edit::Item::Table(tbl));

    if should_default {
        doc["default_location"] = toml_edit::value(a.name.clone());
    }

    write_doc(path, &doc)?;

    if is_update {
        println!("updated location '{}'", a.name);
    } else {
        println!("added location '{}'", a.name);
    }
    if should_default {
        println!("set '{}' as the default location", a.name);
    }
    Ok(())
}

fn remove(path: &Path, a: &LocationNameArg) -> Result<()> {
    let mut doc = read_doc(path)?;
    let removed = doc
        .get_mut("locations")
        .and_then(|i| i.as_table_mut())
        .map(|t| t.remove(&a.name).is_some())
        .unwrap_or(false);
    if !removed {
        bail!("no saved location named '{}'", a.name);
    }

    let cleared_default = doc.get("default_location").and_then(|i| i.as_str()) == Some(&a.name);
    if cleared_default {
        doc.remove("default_location");
    }

    write_doc(path, &doc)?;
    println!("removed location '{}'", a.name);
    if cleared_default {
        println!("(it was the default; no default location is set now)");
    }
    Ok(())
}

fn set_default(path: &Path, a: &LocationNameArg) -> Result<()> {
    let config = config::load(path)?;
    if !config.locations.contains_key(&a.name) {
        bail!("no saved location named '{}'", a.name);
    }
    let mut doc = read_doc(path)?;
    doc["default_location"] = toml_edit::value(a.name.clone());
    write_doc(path, &doc)?;
    println!("default location is now '{}'", a.name);
    Ok(())
}

// ---- helpers ----

fn use_elevation_str(ue: UseElevationArg) -> &'static str {
    match ue {
        UseElevationArg::No => "no",
        UseElevationArg::HanetzShkia => "hanetz-shkia",
        UseElevationArg::All => "all",
    }
}

/// Reads the config file into an editable document, or an empty one if absent.
fn read_doc(path: &Path) -> Result<toml_edit::DocumentMut> {
    match std::fs::read_to_string(path) {
        Ok(s) => s
            .parse::<toml_edit::DocumentMut>()
            .with_context(|| format!("failed to parse config file {}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(toml_edit::DocumentMut::new()),
        Err(e) => Err(e).with_context(|| format!("failed to read config file {}", path.display())),
    }
}

/// Writes the document back to `path`, creating parent directories.
fn write_doc(path: &Path, doc: &toml_edit::DocumentMut) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create config directory {}", parent.display()))?;
    }
    std::fs::write(path, doc.to_string())
        .with_context(|| format!("failed to write config file {}", path.display()))?;
    Ok(())
}

/// Ensures `doc[key]` exists as a table.
fn ensure_table(doc: &mut toml_edit::DocumentMut, key: &str) {
    if doc.get(key).and_then(|i| i.as_table()).is_none() {
        doc[key] = toml_edit::Item::Table(toml_edit::Table::new());
    }
}
