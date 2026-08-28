//! Resolving user-supplied zman names to something computable.
//!
//! Names are matched against [`ComplexZmanimCalendar`](rust_zmanim::prelude::ComplexZmanimCalendar)
//! method names. Hyphens are accepted as underscores; a name containing a `:`
//! is a custom offset spec (see [`crate::custom_zman`]).

use crate::config::Config;
use crate::custom_zman::{self, CustomZman};
use anyhow::{Result, anyhow};
use rust_zmanim::complex_zmanim_calendar::{ALL_ZMANIM, ZmanEntry, ZmanValue, find_zman};
use rust_zmanim::prelude::ComplexZmanimCalendar;

/// Built-in default zman set, used when neither the command line nor the config
/// specifies any zmanim.
pub const DEFAULT_ZMANIM: &[&str] = &[
    "hanetz",
    "sof_zman_shema_gra",
    "chatzos_hayom",
    "mincha_gedola_gra",
    "shkia",
    "tzeis_geonim_8_5_degrees",
];

/// Normalizes a user-supplied name to the canonical method-name form.
pub fn normalize(name: &str) -> String {
    name.trim().to_ascii_lowercase().replace('-', "_")
}

/// Normalizes a custom offset spec. Only the base half gets the usual
/// hyphen-to-underscore treatment: in the offset half a `-` is a minus sign.
fn normalize_spec(spec: &str) -> String {
    match spec.trim().split_once(':') {
        Some((base, offset)) => format!("{}:{}", normalize(base), offset.to_ascii_lowercase()),
        None => normalize(spec),
    }
}

/// Accepts any string, but advertises the registry names as completion candidates.
///
/// Validation deliberately stays in [`resolve_zmanim`]: clap only consults
/// `possible_values` for help rendering and completion generation, never as a
/// validation gate, so hyphen/case normalization, the reserved `:` grammar, and
/// the `did you mean:` suggester all keep working.
#[derive(Debug, Clone)]
pub struct ZmanNameParser;

impl clap::builder::TypedValueParser for ZmanNameParser {
    type Value = String;

    fn parse_ref(
        &self,
        cmd: &clap::Command,
        arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        clap::builder::StringValueParser::new().parse_ref(cmd, arg, value)
    }

    fn possible_values(
        &self,
    ) -> Option<Box<dyn Iterator<Item = clap::builder::PossibleValue> + '_>> {
        Some(Box::new(
            ALL_ZMANIM
                .iter()
                .map(|e| clap::builder::PossibleValue::new(e.name)),
        ))
    }
}

/// One resolved zman: the label to print, plus how to compute it.
#[derive(Debug, Clone)]
pub struct Zman {
    /// The canonical name, used as the column header / JSON key.
    pub name: String,
    /// Where the value comes from.
    pub source: ZmanSource,
}

/// A registry entry, or a custom offset spec.
#[derive(Debug, Clone)]
pub enum ZmanSource {
    Builtin(&'static ZmanEntry),
    Custom(CustomZman),
}

impl Zman {
    /// Computes the zman, returning [`None`] when it does not occur.
    pub fn compute(&self, czc: &ComplexZmanimCalendar) -> Option<ZmanValue> {
        match &self.source {
            ZmanSource::Builtin(entry) => (entry.compute)(czc),
            ZmanSource::Custom(custom) => custom.compute(czc),
        }
    }
}

/// Resolves the requested zman names (or the config/default set when none were
/// given), deduplicating while preserving order.
pub fn resolve_zmanim(requested: &[String], config: &Config) -> Result<Vec<Zman>> {
    let names: Vec<String> = if !requested.is_empty() {
        requested.to_vec()
    } else if let Some(z) = &config.zmanim {
        z.clone()
    } else {
        DEFAULT_ZMANIM.iter().map(|s| (*s).to_string()).collect()
    };

    let mut out = Vec::with_capacity(names.len());
    let mut seen = std::collections::HashSet::new();
    for raw in &names {
        let zman = if raw.contains(':') {
            let norm = normalize_spec(raw);
            Zman {
                source: ZmanSource::Custom(custom_zman::parse(&norm)?),
                name: norm,
            }
        } else {
            let norm = normalize(raw);
            let entry = find_zman(&norm).ok_or_else(|| unknown_zman_error(raw, &norm))?;
            Zman {
                name: entry.name.to_string(),
                source: ZmanSource::Builtin(entry),
            }
        };
        if seen.insert(zman.name.clone()) {
            out.push(zman);
        }
    }
    Ok(out)
}

/// Builds a helpful error for an unknown zman name, including suggestions.
fn unknown_zman_error(raw: &str, norm: &str) -> anyhow::Error {
    let suggestions = suggest(norm);
    if suggestions.is_empty() {
        anyhow!("unknown zman '{raw}' (run 'zmanim list' to see all names)")
    } else {
        let list = suggestions
            .iter()
            .map(|s| format!("  {s}"))
            .collect::<Vec<_>>()
            .join("\n");
        anyhow!("unknown zman '{raw}'\ndid you mean:\n{list}")
    }
}

/// Returns up to 8 registry names related to `query` (substring match, then
/// shared-prefix fallback).
fn suggest(query: &str) -> Vec<&'static str> {
    let mut hits: Vec<&'static str> = ALL_ZMANIM
        .iter()
        .map(|e| e.name)
        .filter(|n| n.contains(query))
        .collect();
    if hits.is_empty() {
        // Fall back to sharing a leading token (e.g. "tzeis").
        let prefix = query.split('_').next().unwrap_or(query);
        if prefix.len() >= 3 {
            hits = ALL_ZMANIM
                .iter()
                .map(|e| e.name)
                .filter(|n| n.contains(prefix))
                .collect();
        }
    }
    hits.truncate(8);
    hits
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_hyphens_and_case() {
        assert_eq!(normalize("Sof-Zman-Shema-GRA"), "sof_zman_shema_gra");
        assert_eq!(normalize("  shkia  "), "shkia");
    }

    #[test]
    fn resolve_known_names() {
        let cfg = Config::default();
        let got = resolve_zmanim(&["shkia".into(), "tzeis-72-minutes".into()], &cfg).unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].name, "shkia");
        assert_eq!(got[1].name, "tzeis_72_minutes");
        assert!(matches!(got[0].source, ZmanSource::Builtin(_)));
    }

    #[test]
    fn dedupe_preserves_first() {
        let cfg = Config::default();
        let got = resolve_zmanim(&["shkia".into(), "shkia".into(), "hanetz".into()], &cfg).unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].name, "shkia");
        assert_eq!(got[1].name, "hanetz");
    }

    #[test]
    fn custom_specs_resolve_alongside_builtins() {
        let cfg = Config::default();
        let got = resolve_zmanim(
            &["shkia".into(), "Alos:18.5deg".into(), "tzeis:72min".into()],
            &cfg,
        )
        .unwrap();
        assert_eq!(got.len(), 3);
        assert!(matches!(got[0].source, ZmanSource::Builtin(_)));
        // Label is the normalized spec, so `Alos` prints as `alos`.
        assert_eq!(got[1].name, "alos:18.5deg");
        assert!(matches!(got[1].source, ZmanSource::Custom(_)));
        assert_eq!(got[2].name, "tzeis:72min");
    }

    #[test]
    fn custom_specs_dedupe_after_normalizing() {
        let cfg = Config::default();
        let got = resolve_zmanim(
            &["TZEIS:72MIN".into(), "tzeis:72min".into(), "shkia".into()],
            &cfg,
        )
        .unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].name, "tzeis:72min");
        assert_eq!(got[1].name, "shkia");
    }

    #[test]
    fn normalize_spec_leaves_the_offset_half_alone() {
        assert_eq!(
            normalize_spec("Sof-Zman-Shema-MGA:16.1DEG"),
            "sof_zman_shema_mga:16.1deg"
        );
        // A `-` in the offset is a minus sign, not a name separator.
        assert_eq!(normalize_spec("alos:-5deg"), "alos:-5deg");
    }

    #[test]
    fn bad_custom_spec_errors() {
        let cfg = Config::default();
        let e = resolve_zmanim(&["alos:18.5xyz".into()], &cfg).unwrap_err();
        assert!(e.to_string().contains("unrecognized offset"));
    }

    #[test]
    fn unknown_name_suggests() {
        let cfg = Config::default();
        let e = resolve_zmanim(&["tzeis".into()], &cfg).unwrap_err();
        let msg = e.to_string();
        assert!(msg.contains("unknown zman"));
        assert!(msg.contains("tzeis")); // suggestions contain tzeis*
    }

    #[test]
    fn empty_uses_builtin_default() {
        let cfg = Config::default();
        let got = resolve_zmanim(&[], &cfg).unwrap();
        assert_eq!(got.len(), DEFAULT_ZMANIM.len());
    }

    #[test]
    fn empty_uses_config_when_present() {
        let cfg = Config {
            zmanim: Some(vec!["shkia".into(), "hanetz".into()]),
            ..Config::default()
        };
        let got = resolve_zmanim(&[], &cfg).unwrap();
        assert_eq!(got.len(), 2);
    }

    #[test]
    fn parser_advertises_every_registry_name() {
        use clap::builder::TypedValueParser;
        let values: Vec<_> = ZmanNameParser
            .possible_values()
            .expect("parser must advertise values")
            .map(|v| v.get_name().to_string())
            .collect();
        assert_eq!(values.len(), ALL_ZMANIM.len());
        assert!(values.iter().any(|v| v == "shkia"));
    }

    /// The whole point of the custom parser: clap must *not* reject unknown
    /// values, so `resolve_zmanim` keeps producing the "did you mean" error and
    /// hyphenated forms keep working.
    #[test]
    fn parser_does_not_reject_unknown_values() {
        use clap::Parser;
        let cli = crate::cli::Cli::try_parse_from(["zmanim", "bogus", "sof-zman-shema-gra"])
            .expect("clap must accept arbitrary zman strings");
        assert_eq!(cli.compute.zmanim, ["bogus", "sof-zman-shema-gra"]);
    }

    #[test]
    fn cli_definition_is_valid() {
        use clap::CommandFactory;
        crate::cli::Cli::command().debug_assert();
    }

    #[test]
    fn all_default_zmanim_are_valid() {
        // Guards against a typo'd default name.
        for name in DEFAULT_ZMANIM {
            assert!(
                find_zman(name).is_some(),
                "default zman '{name}' not in registry"
            );
        }
    }
}
