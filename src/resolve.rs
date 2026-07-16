//! Merges command-line flags (with env already folded in by clap), the config
//! file, and built-in defaults into a single [`ResolvedSettings`].
//!
//! Precedence is flag/env > config > default, applied field by field. Location
//! and timezone are resolved *before* dates, because `today`/`tomorrow` depend
//! on the resolved timezone.

use crate::cli::{ComputeArgs, Format, Precision, Round, TimeStyle};
use crate::config::Config;
use crate::{dates, zman_names};
use anyhow::{Context, Result, bail};
use jiff::{civil::Date, tz::TimeZone};
use rust_zmanim::complex_zmanim_calendar::ZmanEntry;
use rust_zmanim::prelude::{GeoLocation, UseElevation};

/// Fully resolved settings for a compute run.
#[derive(Debug)]
pub struct ResolvedSettings {
    /// The location (with elevation and timezone) to compute for.
    pub geo: GeoLocation,
    /// The elevation policy.
    pub use_elevation: UseElevation,
    /// The dates to compute, in order (non-empty).
    pub dates: Vec<Date>,
    /// The zmanim to compute, in order (deduplicated).
    pub entries: Vec<&'static ZmanEntry>,
    /// Output format.
    pub format: Format,
    /// Display precision.
    pub precision: Precision,
    /// Rounding mode.
    pub round: Round,
    /// Time-rendering style.
    pub time_style: TimeStyle,
}

/// Resolves everything needed for a compute run from the parsed args + config.
pub fn resolve(args: &ComputeArgs, config: &Config) -> Result<ResolvedSettings> {
    let loc = resolve_location(args, config)?;

    let use_elevation: UseElevation = args
        .use_elevation
        .or(loc.use_elevation)
        .map(Into::into)
        .unwrap_or(UseElevation::No);

    let dates = dates::resolve_dates(args.date.as_deref(), args.days, &loc.tz)?;
    let entries = zman_names::resolve_zmanim(&args.zmanim, config)?;

    let format = args.format.or(config.format).unwrap_or(Format::Table);
    let precision = args.precision.or(config.precision).unwrap_or(Precision::M);
    let round = args.round.or(config.round).unwrap_or(Round::Nearest);
    let time_style = args
        .time_style
        .or(config.time_style)
        .unwrap_or(TimeStyle::Auto);

    Ok(ResolvedSettings {
        geo: loc.geo,
        use_elevation,
        dates,
        entries,
        format,
        precision,
        round,
        time_style,
    })
}

/// Intermediate result of location resolution.
struct ResolvedLocation {
    geo: GeoLocation,
    tz: TimeZone,
    /// Per-location elevation policy (only for saved locations).
    use_elevation: Option<crate::cli::UseElevationArg>,
}

fn resolve_location(args: &ComputeArgs, config: &Config) -> Result<ResolvedLocation> {
    // Raw coordinates mode (clap guarantees lat and lon come together and
    // conflict with --location).
    if let (Some(lat), Some(lon)) = (args.lat, args.lon) {
        let tz = match &args.tz {
            Some(s) => parse_tz(s)?,
            None => TimeZone::system(),
        };
        let elevation = args.elevation.unwrap_or(0.0);
        let geo = GeoLocation::new(lat, lon, elevation, tz.clone())
            .map_err(|e| anyhow::anyhow!("invalid location: {e}"))?;
        return Ok(ResolvedLocation {
            geo,
            tz,
            use_elevation: None,
        });
    }

    // Named location: explicit --location, else default_location from config.
    let name = args
        .location
        .clone()
        .or_else(|| config.default_location.clone());
    let Some(name) = name else {
        bail!(
            "no location given\n\
             provide one of:\n  \
             --lat <LAT> --lon <LON> [--tz <TZ>]\n  \
             --location <NAME>   (see 'zmanim locations')\n\
             or set a default with 'zmanim locations set-default <NAME>'"
        );
    };

    let entry = config.locations.get(&name).ok_or_else(|| {
        let mut available: Vec<&String> = config.locations.keys().collect();
        available.sort();
        if available.is_empty() {
            anyhow::anyhow!(
                "unknown location '{name}' (no locations saved; add one with 'zmanim locations add')"
            )
        } else {
            let list = available
                .iter()
                .map(|n| format!("  {n}"))
                .collect::<Vec<_>>()
                .join("\n");
            anyhow::anyhow!("unknown location '{name}'\navailable locations:\n{list}")
        }
    })?;

    // --tz overrides the saved timezone.
    let tz = match &args.tz {
        Some(s) => parse_tz(s)?,
        None => parse_tz(&entry.timezone)
            .with_context(|| format!("saved location '{name}' has an invalid timezone"))?,
    };
    let elevation = args.elevation.or(entry.elevation).unwrap_or(0.0);
    let geo = GeoLocation::new(entry.latitude, entry.longitude, elevation, tz.clone())
        .map_err(|e| anyhow::anyhow!("saved location '{name}' is invalid: {e}"))?;

    Ok(ResolvedLocation {
        geo,
        tz,
        use_elevation: entry.use_elevation,
    })
}

/// Parses an IANA timezone name, echoing the bad input on failure.
fn parse_tz(s: &str) -> Result<TimeZone> {
    TimeZone::get(s).map_err(|_| {
        anyhow::anyhow!("unknown timezone '{s}' (expected an IANA name like 'Asia/Jerusalem')")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::UseElevationArg;
    use crate::config::LocationEntry;

    fn base_args() -> ComputeArgs {
        ComputeArgs {
            zmanim: vec![],
            location: None,
            lat: None,
            lon: None,
            elevation: None,
            tz: None,
            date: Some("2026-07-14".into()),
            days: None,
            use_elevation: None,
            format: None,
            precision: None,
            round: None,
            time_style: None,
        }
    }

    fn config_with_home() -> Config {
        let mut cfg = Config::default();
        cfg.locations.insert(
            "home".into(),
            LocationEntry {
                latitude: 31.778,
                longitude: 35.235,
                elevation: Some(754.0),
                timezone: "Asia/Jerusalem".into(),
                use_elevation: Some(UseElevationArg::HanetzShkia),
            },
        );
        cfg
    }

    #[test]
    fn raw_coords_resolve() {
        let mut args = base_args();
        args.lat = Some(40.0);
        args.lon = Some(-74.0);
        args.tz = Some("America/New_York".into());
        let cfg = Config::default();
        let r = resolve(&args, &cfg).unwrap();
        assert!((r.geo.latitude() - 40.0).abs() < 1e-9);
        assert_eq!(r.use_elevation, UseElevation::No);
    }

    #[test]
    fn saved_location_resolves_with_its_tz_and_policy() {
        let mut args = base_args();
        args.location = Some("home".into());
        let cfg = config_with_home();
        let r = resolve(&args, &cfg).unwrap();
        assert_eq!(*r.geo.timezone(), TimeZone::get("Asia/Jerusalem").unwrap());
        assert_eq!(r.use_elevation, UseElevation::HanetzShkia);
        assert!((r.geo.elevation() - 754.0).abs() < 1e-9);
    }

    #[test]
    fn flag_tz_overrides_saved_tz() {
        let mut args = base_args();
        args.location = Some("home".into());
        args.tz = Some("UTC".into());
        let cfg = config_with_home();
        let r = resolve(&args, &cfg).unwrap();
        assert_eq!(*r.geo.timezone(), TimeZone::UTC);
    }

    #[test]
    fn flag_use_elevation_beats_location_policy() {
        let mut args = base_args();
        args.location = Some("home".into());
        args.use_elevation = Some(UseElevationArg::All);
        let cfg = config_with_home();
        let r = resolve(&args, &cfg).unwrap();
        assert_eq!(r.use_elevation, UseElevation::All);
    }

    #[test]
    fn default_location_used_when_no_flag() {
        let mut args = base_args();
        let mut cfg = config_with_home();
        cfg.default_location = Some("home".into());
        args.location = None;
        let r = resolve(&args, &cfg).unwrap();
        assert_eq!(*r.geo.timezone(), TimeZone::get("Asia/Jerusalem").unwrap());
    }

    #[test]
    fn no_location_errors() {
        let args = base_args();
        let cfg = Config::default();
        let e = resolve(&args, &cfg).unwrap_err();
        assert!(e.to_string().contains("no location given"));
    }

    #[test]
    fn unknown_location_lists_available() {
        let mut args = base_args();
        args.location = Some("nope".into());
        let cfg = config_with_home();
        let e = resolve(&args, &cfg).unwrap_err();
        let msg = e.to_string();
        assert!(msg.contains("unknown location"));
        assert!(msg.contains("home"));
    }

    #[test]
    fn output_defaults_and_overrides() {
        let mut args = base_args();
        args.lat = Some(0.0);
        args.lon = Some(0.0);
        args.tz = Some("UTC".into());
        let cfg = Config {
            format: Some(Format::Csv),
            ..Config::default()
        };
        // config supplies csv; flag (None) falls through to it
        let r = resolve(&args, &cfg).unwrap();
        assert_eq!(r.format, Format::Csv);
        // flag beats config
        args.format = Some(Format::Json);
        let r = resolve(&args, &cfg).unwrap();
        assert_eq!(r.format, Format::Json);
    }
}
