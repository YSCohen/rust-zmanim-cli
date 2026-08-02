//! Command-line interface definition (clap) and the option enums shared
//! between the CLI, the config file, and output rendering.

use crate::zman_names::ZmanNameParser;
use clap::{Args, Parser, Subcommand, ValueEnum};
use rust_zmanim::prelude::UseElevation;
use serde::Deserialize;
use std::path::PathBuf;

const EXAMPLES: &str = "\
EXAMPLES:
  # Compute zmanim for raw coordinates, today
  zmanim shkia tzeis_72_minutes --lat 31.778 --lon 35.235 --tz Asia/Jerusalem

  # Use a saved location and a date range
  zmanim sof_zman_shema_gra shkia --location home --date 2026-07-14..2026-07-20

  # Default zman set (from config), as JSON, next 7 days
  zmanim --location home --days 7 --format json

  # Discover zman names
  zmanim list geonim

  # Save a location
  zmanim locations add home --lat 31.778 --lon 35.235 --tz Asia/Jerusalem
";

/// A command-line tool for computing Jewish *zmanim*.
///
/// Running `zmanim` with no subcommand computes zmanim for the given (or
/// configured) location and date(s). Use the subcommands to discover zman
/// names, manage saved locations, and generate shell completions.
#[derive(Debug, Parser)]
#[command(name = "zmanim", version, about, long_about = None, after_help = EXAMPLES)]
pub struct Cli {
    /// Utility subcommand; omit to compute zmanim.
    #[command(subcommand)]
    pub command: Option<Command>,

    #[command(flatten)]
    pub compute: ComputeArgs,

    /// Path to the config file (overrides the default location).
    #[arg(
        long,
        global = true,
        value_name = "PATH",
        env = "ZMANIM_CONFIG",
        hide = true
    )]
    pub config: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// List available zman names (optionally filtered by a substring).
    List(ListArgs),
    /// Manage saved locations.
    Locations(LocationsArgs),
    /// Generate a shell completion script.
    Completions(CompletionsArgs),
}

/// Arguments for computing zmanim (used when no subcommand is given).
#[derive(Debug, Args)]
pub struct ComputeArgs {
    /// Zmanim to compute, by name (see `zmanim list`). Hyphens and underscores
    /// are interchangeable. If omitted, a default set from config is used.
    #[arg(
        value_name = "ZMAN",
        value_parser = ZmanNameParser,
        hide_possible_values = true
    )]
    pub zmanim: Vec<String>,

    /// Saved location name (see `zmanim locations`).
    #[arg(long, short = 'L', value_name = "NAME", env = "ZMANIM_LOCATION",
          conflicts_with_all = ["lat", "lon"])]
    pub location: Option<String>,

    /// Latitude in decimal degrees (requires --lon).
    #[arg(long, allow_negative_numbers = true, requires = "lon")]
    pub lat: Option<f64>,

    /// Longitude in decimal degrees (requires --lat).
    #[arg(long, allow_negative_numbers = true, requires = "lat")]
    pub lon: Option<f64>,

    /// Elevation in meters (default 0).
    #[arg(long, value_name = "METERS")]
    pub elevation: Option<f64>,

    /// IANA timezone (e.g. Asia/Jerusalem). Defaults to the location's saved
    /// timezone, or the system timezone for raw coordinates.
    #[arg(long, value_name = "TZ", env = "ZMANIM_TZ")]
    pub tz: Option<String>,

    /// Date, `today`, `tomorrow`, or an inclusive range `START..END`.
    #[arg(long, value_name = "DATE")]
    pub date: Option<String>,

    /// Number of consecutive days starting from --date (or today). Cannot be
    /// combined with a `START..END` range.
    #[arg(long, value_name = "N")]
    pub days: Option<u32>,

    /// Elevation policy for the calculations.
    #[arg(long, value_enum, value_name = "MODE", env = "ZMANIM_USE_ELEVATION")]
    pub use_elevation: Option<UseElevationArg>,

    /// Output format.
    #[arg(long, value_enum, env = "ZMANIM_FORMAT")]
    pub format: Option<Format>,

    /// Display precision for human-readable times/durations.
    #[arg(long, value_enum, env = "ZMANIM_PRECISION")]
    pub precision: Option<Precision>,

    /// Rounding mode for human-readable times/durations.
    #[arg(long, value_enum, env = "ZMANIM_ROUND")]
    pub round: Option<Round>,

    /// Time rendering: `auto` (human for table/csv, ISO for json), or force
    /// `human`/`iso` in any format.
    #[arg(
        long = "time-style",
        value_enum,
        value_name = "STYLE",
        env = "ZMANIM_TIME_STYLE"
    )]
    pub time_style: Option<TimeStyle>,
}

#[derive(Debug, Args)]
pub struct ListArgs {
    /// Only show names containing this substring.
    #[arg(
        value_name = "FILTER",
        value_parser = ZmanNameParser,
        hide_possible_values = true
    )]
    pub filter: Option<String>,

    /// Only show zmanim of this kind.
    #[arg(long, value_enum)]
    pub kind: Option<KindArg>,
}

#[derive(Debug, Args)]
pub struct LocationsArgs {
    #[command(subcommand)]
    pub command: Option<LocationsCommand>,
}

#[derive(Debug, Subcommand)]
pub enum LocationsCommand {
    /// List saved locations (the default when no action is given).
    List,
    /// Add or update a saved location.
    Add(LocationAddArgs),
    /// Remove a saved location.
    Remove(LocationNameArg),
    /// Set the default location.
    SetDefault(LocationNameArg),
}

#[derive(Debug, Args)]
pub struct LocationAddArgs {
    /// Name to save the location under.
    pub name: String,
    /// Latitude in decimal degrees.
    #[arg(long, allow_negative_numbers = true)]
    pub lat: f64,
    /// Longitude in decimal degrees.
    #[arg(long, allow_negative_numbers = true)]
    pub lon: f64,
    /// Elevation in meters (default 0).
    #[arg(long, value_name = "METERS")]
    pub elevation: Option<f64>,
    /// IANA timezone (e.g. Asia/Jerusalem).
    #[arg(long, value_name = "TZ")]
    pub tz: String,
    /// Elevation policy to use with this location.
    #[arg(long, value_enum, value_name = "MODE")]
    pub use_elevation: Option<UseElevationArg>,
    /// Also set this as the default location.
    #[arg(long)]
    pub default: bool,
}

#[derive(Debug, Args)]
pub struct LocationNameArg {
    /// Location name.
    pub name: String,
}

#[derive(Debug, Args)]
pub struct CompletionsArgs {
    /// Shell to generate completions for.
    #[arg(value_enum)]
    pub shell: CompletionShell,
}

/// Shell to generate completions for. Wraps `clap_complete::Shell`, which has
/// no Nushell variant, with the generator from `clap_complete_nushell`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CompletionShell {
    Bash,
    Elvish,
    Fish,
    Nushell,
    #[value(name = "powershell")]
    PowerShell,
    Zsh,
}

// ---- Shared option enums (clap ValueEnum + serde Deserialize) ----

/// Output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Deserialize)]
#[serde(rename_all = "kebab-case")] // lowercase
pub enum Format {
    /// Aligned text table (or a vertical list for a single date).
    Table,
    /// Comma-separated values.
    Csv,
    /// JSON array of per-date objects.
    Json,
}

/// Display precision for human-readable output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Precision {
    /// Minutes.
    M,
    /// Seconds.
    S,
    /// Milliseconds.
    Ms,
}

/// Rounding mode for human-readable output. `down`/`up` mean earlier/later.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Round {
    /// Round to the nearest unit.
    Nearest,
    /// Round toward earlier times (floor).
    Down,
    /// Round toward later times (ceil).
    Up,
}

/// How time values are rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TimeStyle {
    /// Human clock times for table/csv, full-precision ISO for json.
    Auto,
    /// Human-readable clock times/durations (honors precision and round).
    Human,
    /// Full-precision ISO 8601 (RFC-3339 times, `PT..` durations).
    Iso,
}

/// Elevation policy, mirroring [`UseElevation`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UseElevationArg {
    /// Never use elevation.
    No,
    /// Use elevation only for sunrise/sunset.
    HanetzShkia,
    /// Use elevation for all zmanim.
    All,
}

impl From<UseElevationArg> for UseElevation {
    fn from(a: UseElevationArg) -> Self {
        match a {
            UseElevationArg::No => UseElevation::No,
            UseElevationArg::HanetzShkia => UseElevation::HanetzShkia,
            UseElevationArg::All => UseElevation::All,
        }
    }
}

/// Zman kind filter for `zmanim list`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum KindArg {
    /// Instant-in-time zmanim.
    Time,
    /// Duration zmanim (*shaah zmanis*).
    Duration,
}
