//! Loading and locating the TOML config file.
//!
//! The config supplies defaults that sit below environment variables and
//! command-line flags in precedence. Mutation (for `zmanim locations add`
//! etc.) is handled in `commands::locations`.

use crate::cli::{Format, Precision, Round, TimeStyle, UseElevationArg};
use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The deserialized config file.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Location name used when neither `--location` nor raw coords are given.
    pub default_location: Option<String>,
    /// Default zman names used when none are given on the command line.
    pub zmanim: Option<Vec<String>>,
    /// Default output format.
    pub format: Option<Format>,
    /// Default display precision.
    pub precision: Option<Precision>,
    /// Default rounding mode.
    pub round: Option<Round>,
    /// Default time-rendering style.
    pub time_style: Option<TimeStyle>,
    /// Saved locations, keyed by name.
    #[serde(default)]
    pub locations: BTreeMap<String, LocationEntry>,
}

/// A single saved location.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocationEntry {
    /// Latitude in decimal degrees.
    pub latitude: f64,
    /// Longitude in decimal degrees.
    pub longitude: f64,
    /// Elevation in meters (default 0).
    pub elevation: Option<f64>,
    /// IANA timezone name.
    pub timezone: String,
    /// Per-location elevation policy.
    pub use_elevation: Option<UseElevationArg>,
}

/// Resolves the config file path: the override (from `--config`/`ZMANIM_CONFIG`)
/// if given, otherwise the XDG config location `~/.config/zmanim/config.toml`.
pub fn config_path(override_path: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(p) = override_path {
        return Ok(p);
    }
    use etcetera::BaseStrategy as _;
    let strategy =
        etcetera::choose_base_strategy().context("could not determine the config directory")?;
    Ok(strategy.config_dir().join("zmanim").join("config.toml"))
}

/// Loads the config from `path`. A missing file yields the default config; a
/// malformed file is a hard error naming the path.
pub fn load(path: &Path) -> Result<Config> {
    match std::fs::read_to_string(path) {
        Ok(contents) => toml::from_str(&contents)
            .with_context(|| format!("failed to parse config file {}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
        Err(e) => Err(e).with_context(|| format!("failed to read config file {}", path.display())),
    }
}
