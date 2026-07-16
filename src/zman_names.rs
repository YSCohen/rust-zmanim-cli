//! Resolving user-supplied zman names to registry entries.
//!
//! Names are matched against [`ComplexZmanimCalendar`](rust_zmanim::prelude::ComplexZmanimCalendar)
//! method names. Hyphens are accepted as underscores; a `:` is reserved for a
//! future custom-offset grammar and rejected for now.

use crate::config::Config;
use anyhow::{Result, anyhow, bail};
use rust_zmanim::complex_zmanim_calendar::{ALL_ZMANIM, ZmanEntry, find_zman};

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

/// Resolves the requested zman names (or the config/default set when none were
/// given) into registry entries, deduplicating while preserving order.
pub fn resolve_zmanim(requested: &[String], config: &Config) -> Result<Vec<&'static ZmanEntry>> {
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
        if raw.contains(':') {
            // coming soon...
            bail!("custom offsets (e.g. 'alos:18.5deg') are not supported yet: '{raw}'");
        }
        let norm = normalize(raw);
        let entry = find_zman(&norm).ok_or_else(|| unknown_zman_error(raw, &norm))?;
        if seen.insert(entry.name) {
            out.push(entry);
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
    fn colon_is_reserved() {
        let cfg = Config::default();
        let e = resolve_zmanim(&["alos:18.5deg".into()], &cfg).unwrap_err();
        assert!(e.to_string().contains("not supported yet"));
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
