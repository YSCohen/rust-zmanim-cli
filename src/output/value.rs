//! Rendering [`ZmanValue`]s to strings, honoring precision, rounding, and
//! time-style.

use crate::cli::{Format, Precision, Round, TimeStyle};
use jiff::{RoundMode, SignedDuration, SignedDurationRound, Unit, Zoned, ZonedRound};
use rust_zmanim::complex_zmanim_calendar::ZmanValue;

/// The marker shown in place of a zman that does not occur (table output).
pub const MISSING: &str = "-";

/// The concrete rendering to use, after resolving [`TimeStyle::Auto`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderStyle {
    /// Human-readable clock times/durations (honors precision + round).
    Human,
    /// Full-precision ISO 8601.
    Iso,
}

/// Resolves `Auto` against the output format: human for table/csv, ISO for
/// json.
pub fn resolve_style(style: TimeStyle, format: Format) -> RenderStyle {
    match style {
        TimeStyle::Human => RenderStyle::Human,
        TimeStyle::Iso => RenderStyle::Iso,
        TimeStyle::Auto => match format {
            Format::Json => RenderStyle::Iso,
            Format::Table | Format::Csv => RenderStyle::Human,
        },
    }
}

/// Renders a value to a display string (used by table and csv).
pub fn render(value: &ZmanValue, style: RenderStyle, precision: Precision, round: Round) -> String {
    match style {
        RenderStyle::Human => match value {
            ZmanValue::Time(z) => human_time(z, precision, round),
            ZmanValue::Duration(d) => human_duration(d, precision, round),
        },
        RenderStyle::Iso => match value {
            ZmanValue::Time(z) => iso_time(z),
            ZmanValue::Duration(d) => iso_duration(d),
        },
    }
}

fn unit_of(precision: Precision) -> Unit {
    match precision {
        Precision::M => Unit::Minute,
        Precision::S => Unit::Second,
        Precision::Ms => Unit::Millisecond,
    }
}

fn mode_of(round: Round) -> RoundMode {
    match round {
        Round::Nearest => RoundMode::HalfExpand,
        Round::Down => RoundMode::Floor,
        Round::Up => RoundMode::Ceil,
    }
}

/// A rounded clock time. Falls back to the unrounded value if rounding fails.
fn human_time(z: &Zoned, precision: Precision, round: Round) -> String {
    let opts = ZonedRound::new()
        .smallest(unit_of(precision))
        .mode(mode_of(round));
    let z = z.round(opts).unwrap_or_else(|_| z.clone());
    match precision {
        Precision::M => z.strftime("%H:%M").to_string(),
        Precision::S => z.strftime("%H:%M:%S").to_string(),
        Precision::Ms => z.strftime("%H:%M:%S%.3f").to_string(),
    }
}

/// A rounded clock-style duration, e.g. `1:09`, `1:09:26`, `1:09:26.041`.
fn human_duration(d: &SignedDuration, precision: Precision, round: Round) -> String {
    let opts = SignedDurationRound::new()
        .smallest(unit_of(precision))
        .mode(mode_of(round));
    let d = d.round(opts).unwrap_or(*d);

    let sign = if d.is_negative() { "-" } else { "" };
    let total_secs = d.as_secs().unsigned_abs();
    let millis = d.subsec_nanos().unsigned_abs() / 1_000_000;
    let h = total_secs / 3600;
    let m = (total_secs % 3600) / 60;
    let s = total_secs % 60;
    match precision {
        Precision::M => format!("{sign}{h}:{m:02}"),
        Precision::S => format!("{sign}{h}:{m:02}:{s:02}"),
        Precision::Ms => format!("{sign}{h}:{m:02}:{s:02}.{millis:03}"),
    }
}

/// Full-precision RFC-3339 timestamp with numeric offset.
pub fn iso_time(z: &Zoned) -> String {
    z.strftime("%Y-%m-%dT%H:%M:%S%.f%:z").to_string()
}

/// Full-precision ISO-8601 duration (`PT..`), via `SignedDuration`'s Display.
pub fn iso_duration(d: &SignedDuration) -> String {
    d.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::{civil, tz::TimeZone};

    fn sample_time() -> Zoned {
        civil::date(2026, 7, 14)
            .at(19, 47, 32, 123_456_789)
            .to_zoned(TimeZone::get("Asia/Jerusalem").unwrap())
            .unwrap()
    }

    #[test]
    fn human_time_precisions_nearest() {
        let z = sample_time();
        assert_eq!(human_time(&z, Precision::M, Round::Nearest), "19:48");
        assert_eq!(human_time(&z, Precision::S, Round::Nearest), "19:47:32");
        assert_eq!(
            human_time(&z, Precision::Ms, Round::Nearest),
            "19:47:32.123"
        );
    }

    #[test]
    fn human_time_round_directions() {
        let z = sample_time(); // 19:47:32
        assert_eq!(human_time(&z, Precision::M, Round::Down), "19:47");
        assert_eq!(human_time(&z, Precision::M, Round::Up), "19:48");
    }

    #[test]
    fn human_duration_format() {
        let d = SignedDuration::new(4166, 40_000_000); // 1h 9m 26.04s
        assert_eq!(human_duration(&d, Precision::M, Round::Nearest), "1:09");
        assert_eq!(human_duration(&d, Precision::S, Round::Nearest), "1:09:26");
        assert_eq!(
            human_duration(&d, Precision::Ms, Round::Nearest),
            "1:09:26.040"
        );
    }

    #[test]
    fn iso_time_is_full_precision() {
        let z = sample_time();
        assert_eq!(iso_time(&z), "2026-07-14T19:47:32.123456789+03:00");
    }

    #[test]
    fn iso_duration_is_pt() {
        let d = SignedDuration::new(4166, 40_000_000);
        assert_eq!(iso_duration(&d), "PT1H9M26.04S");
    }

    #[test]
    fn resolve_style_auto() {
        assert_eq!(
            resolve_style(TimeStyle::Auto, Format::Table),
            RenderStyle::Human
        );
        assert_eq!(
            resolve_style(TimeStyle::Auto, Format::Csv),
            RenderStyle::Human
        );
        assert_eq!(
            resolve_style(TimeStyle::Auto, Format::Json),
            RenderStyle::Iso
        );
        assert_eq!(
            resolve_style(TimeStyle::Iso, Format::Table),
            RenderStyle::Iso
        );
        assert_eq!(
            resolve_style(TimeStyle::Human, Format::Json),
            RenderStyle::Human
        );
    }
}
