//! The custom offset grammar: `<base>:<value><unit>`, e.g. `alos:18.5deg`,
//! `shaah_zmanis_mga:80min`, `tzeis:100minz`.
//!
//! Every built-in `alos_*` / `tzeis_*` / `*_mga_*` zman is one of ten
//! [`ComplexZmanimCalendar`] methods called with a hardcoded
//! [`ZmanOffset`]; this module lets the user supply that offset directly.

use anyhow::{Result, anyhow, bail};
use rust_zmanim::complex_zmanim_calendar::ZmanValue;
use rust_zmanim::prelude::{ComplexZmanimCalendar, ZmanOffset};

/// A zman family that takes a [`ZmanOffset`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Base {
    Alos,
    Tzeis,
    ShaahZmanisMga,
    SofZmanShemaMga,
    SofZmanTefilaMga,
    SofZmanBiurChametzMga,
    MinchaGedolaMga,
    SamuchLeminchaKetanaMga,
    MinchaKetanaMga,
    PlagMga,
}

/// The accepted base names, in the order they are listed back to the user.
const BASES: &[(&str, Base)] = &[
    ("alos", Base::Alos),
    ("tzeis", Base::Tzeis),
    ("shaah_zmanis_mga", Base::ShaahZmanisMga),
    ("sof_zman_shema_mga", Base::SofZmanShemaMga),
    ("sof_zman_tefila_mga", Base::SofZmanTefilaMga),
    ("sof_zman_biur_chametz_mga", Base::SofZmanBiurChametzMga),
    ("mincha_gedola_mga", Base::MinchaGedolaMga),
    ("samuch_lemincha_ketana_mga", Base::SamuchLeminchaKetanaMga),
    ("mincha_ketana_mga", Base::MinchaKetanaMga),
    ("plag_mga", Base::PlagMga),
];

/// The parsed offset. Mirrors [`ZmanOffset`], except that the *shaah zmanis*
/// backing `minz` is only known once there is a calendar to ask.
#[derive(Debug, Clone, Copy, PartialEq)]
enum OffsetSpec {
    Degrees(f64),
    Minutes(f64),
    MinutesZmaniyos(f64),
}

/// A zman built from a user-supplied offset rather than the registry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CustomZman {
    base: Base,
    offset: OffsetSpec,
}

/// Parses a normalized (see [`normalize`](crate::zman_names::normalize)) spec
/// of the form `<base>:<value><unit>`.
pub fn parse(spec: &str) -> Result<CustomZman> {
    let (base, rest) = spec
        .split_once(':')
        .ok_or_else(|| anyhow!("custom zman '{spec}' is missing a ':'"))?;

    let base = BASES
        .iter()
        .find(|(name, _)| *name == base)
        .map(|(_, b)| *b)
        .ok_or_else(|| unknown_base_error(spec))?;

    let offset = if let Some(n) = rest.strip_suffix("minz") {
        OffsetSpec::MinutesZmaniyos(number(spec, n)?)
    } else if let Some(n) = rest.strip_suffix("min") {
        OffsetSpec::Minutes(number(spec, n)?)
    } else if let Some(n) = rest.strip_suffix("deg") {
        let degrees = number(spec, n)?;
        if degrees.abs() >= 90.0 {
            bail!("custom zman '{spec}': degrees must be between -90 and 90");
        }
        OffsetSpec::Degrees(degrees)
    } else {
        bail!(
            "custom zman '{spec}' has an unrecognized offset '{rest}'\n\
             expected a number followed by 'deg', 'min', or 'minz'"
        );
    };

    Ok(CustomZman { base, offset })
}

/// Parses the numeric part of an offset. Negatives are allowed: they flip the
/// direction, so `alos:-72min` is 72 minutes *after* sunrise and `alos:-5deg`
/// is the sun 5&deg; *above* the horizon.
fn number(spec: &str, text: &str) -> Result<f64> {
    let value: f64 = text
        .parse()
        .map_err(|_| anyhow!("custom zman '{spec}' has an invalid number '{text}'"))?;
    if !value.is_finite() {
        bail!("custom zman '{spec}' needs a finite offset");
    }
    Ok(value)
}

fn unknown_base_error(spec: &str) -> anyhow::Error {
    let list = BASES
        .iter()
        .map(|(name, _)| format!("  {name}"))
        .collect::<Vec<_>>()
        .join("\n");
    anyhow!("unknown custom zman base in '{spec}'\nvalid bases:\n{list}")
}

impl CustomZman {
    /// Computes the zman, returning [`None`] when the underlying solar event
    /// does not occur.
    pub fn compute(&self, czc: &ComplexZmanimCalendar) -> Option<ZmanValue> {
        let offset = match self.offset {
            OffsetSpec::Degrees(d) => ZmanOffset::Degrees(d),
            OffsetSpec::Minutes(m) => ZmanOffset::Minutes(m),
            // The library's own `*_minutes_zmanis` zmanim all measure minutes
            // zmaniyos against the GRA (sunrise-to-sunset) shaah zmanis.
            OffsetSpec::MinutesZmaniyos(m) => ZmanOffset::MinutesZmaniyos {
                minutes_zmaniyos: m,
                shaah_zmanis: czc.shaah_zmanis_gra()?,
            },
        };

        match self.base {
            Base::Alos => czc.alos(&offset).map(ZmanValue::Time),
            Base::Tzeis => czc.tzeis(&offset).map(ZmanValue::Time),
            Base::ShaahZmanisMga => czc.shaah_zmanis_mga(&offset).map(ZmanValue::Duration),
            Base::SofZmanShemaMga => czc.sof_zman_shema_mga(&offset).map(ZmanValue::Time),
            Base::SofZmanTefilaMga => czc.sof_zman_tefila_mga(&offset).map(ZmanValue::Time),
            Base::SofZmanBiurChametzMga => {
                czc.sof_zman_biur_chametz_mga(&offset).map(ZmanValue::Time)
            }
            Base::MinchaGedolaMga => czc.mincha_gedola_mga(&offset).map(ZmanValue::Time),
            Base::SamuchLeminchaKetanaMga => {
                czc.samuch_lemincha_ketana_mga(&offset).map(ZmanValue::Time)
            }
            Base::MinchaKetanaMga => czc.mincha_ketana_mga(&offset).map(ZmanValue::Time),
            Base::PlagMga => czc.plag_mga(&offset).map(ZmanValue::Time),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_each_unit() {
        assert_eq!(
            parse("alos:18.5deg").unwrap(),
            CustomZman {
                base: Base::Alos,
                offset: OffsetSpec::Degrees(18.5)
            }
        );
        assert_eq!(
            parse("tzeis:72min").unwrap(),
            CustomZman {
                base: Base::Tzeis,
                offset: OffsetSpec::Minutes(72.0)
            }
        );
        assert_eq!(
            parse("tzeis:90minz").unwrap(),
            CustomZman {
                base: Base::Tzeis,
                offset: OffsetSpec::MinutesZmaniyos(90.0)
            }
        );
    }

    #[test]
    fn parses_every_base() {
        for (name, base) in BASES {
            let spec = format!("{name}:72min");
            assert_eq!(parse(&spec).unwrap().base, *base);
        }
    }

    #[test]
    fn unknown_base_lists_valid_ones() {
        let e = parse("bogus:72min").unwrap_err().to_string();
        assert!(e.contains("unknown custom zman base"));
        assert!(e.contains("samuch_lemincha_ketana_mga"));
    }

    #[test]
    fn unknown_unit_names_the_valid_ones() {
        let e = parse("alos:18.5xyz").unwrap_err().to_string();
        assert!(e.contains("'deg'"));
        assert!(e.contains("'min'"));
        assert!(e.contains("'minz'"));
    }

    #[test]
    fn rejects_bad_numbers() {
        assert!(parse("alos:deg").unwrap_err().to_string().contains("''"));
        assert!(
            parse("alos:abcdeg")
                .unwrap_err()
                .to_string()
                .contains("invalid number")
        );
        assert!(
            parse("alos:infdeg")
                .unwrap_err()
                .to_string()
                .contains("finite")
        );
        assert!(
            parse("alos:nandeg")
                .unwrap_err()
                .to_string()
                .contains("finite")
        );
    }

    #[test]
    fn accepts_negative_offsets() {
        assert_eq!(
            parse("alos:-72min").unwrap(),
            CustomZman {
                base: Base::Alos,
                offset: OffsetSpec::Minutes(-72.0)
            }
        );
        assert_eq!(
            parse("alos:-5deg").unwrap(),
            CustomZman {
                base: Base::Alos,
                offset: OffsetSpec::Degrees(-5.0)
            }
        );
        assert_eq!(
            parse("tzeis:-90minz").unwrap(),
            CustomZman {
                base: Base::Tzeis,
                offset: OffsetSpec::MinutesZmaniyos(-90.0)
            }
        );
    }

    #[test]
    fn rejects_degrees_outside_plus_minus_90() {
        for spec in ["alos:90deg", "alos:-90deg", "alos:120deg"] {
            assert!(
                parse(spec)
                    .unwrap_err()
                    .to_string()
                    .contains("between -90 and 90"),
                "{spec}"
            );
        }
        assert!(parse("alos:89.9deg").is_ok());
        assert!(parse("alos:-89.9deg").is_ok());
    }

    #[test]
    fn empty_base_is_unknown() {
        assert!(
            parse(":72min")
                .unwrap_err()
                .to_string()
                .contains("unknown custom zman base")
        );
    }
}
