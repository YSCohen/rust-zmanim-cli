//! `zmanim list [FILTER]` - print available zman names.

use crate::cli::{KindArg, ListArgs};
use crate::zman_names::normalize;
use rust_zmanim::complex_zmanim_calendar::{ALL_ZMANIM, ZmanKind};

/// Prints matching zman names, one per line, sorted alphabetically.
pub fn run(args: &ListArgs) {
    let filter = args.filter.as_deref().map(normalize);

    let mut names: Vec<&'static str> = ALL_ZMANIM
        .iter()
        .filter(|e| match args.kind {
            None => true,
            Some(KindArg::Time) => e.kind == ZmanKind::Time,
            Some(KindArg::Duration) => e.kind == ZmanKind::Duration,
        })
        .filter(|e| match &filter {
            None => true,
            Some(f) => e.name.contains(f.as_str()),
        })
        .map(|e| e.name)
        .collect();

    names.sort_unstable();
    for name in names {
        println!("{name}");
    }
}
