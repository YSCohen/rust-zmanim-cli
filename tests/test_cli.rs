//! End-to-end integration tests driving the `zmanim` binary.

use assert_cmd::Command;
use predicates::prelude::*;
use std::path::PathBuf;
use tempfile::TempDir;

/// A `zmanim` command with an isolated (empty) config directory.
fn zmanim(tmp: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("zmanim").unwrap();
    cmd.env("ZMANIM_CONFIG", tmp.path().join("config.toml"));
    // Neutralize any inherited env that could leak into resolution.
    for var in [
        "ZMANIM_LOCATION",
        "ZMANIM_TZ",
        "ZMANIM_FORMAT",
        "ZMANIM_PRECISION",
        "ZMANIM_ROUND",
        "ZMANIM_TIME_STYLE",
        "ZMANIM_USE_ELEVATION",
    ] {
        cmd.env_remove(var);
    }
    cmd
}

const JLM: [&str; 6] = [
    "--lat",
    "31.778",
    "--lon",
    "35.235",
    "--tz",
    "Asia/Jerusalem",
];

#[test]
fn single_date_table() {
    let tmp = TempDir::new().unwrap();
    zmanim(&tmp)
        .args(["shkia", "hanetz"])
        .args(JLM)
        .args(["--date", "2026-07-14"])
        .assert()
        .success()
        .stdout(predicate::str::contains("shkia"))
        .stdout(predicate::str::contains("hanetz"));
}

#[test]
fn multi_date_csv_shape() {
    let tmp = TempDir::new().unwrap();
    let out = zmanim(&tmp)
        .args(["shkia", "tzeis_72_minutes"])
        .args(JLM)
        .args(["--date", "2026-07-14..2026-07-16", "--format", "csv"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines[0], "date,shkia,tzeis_72_minutes");
    assert_eq!(lines.len(), 4); // header + 3 dates
    assert!(lines[1].starts_with("2026-07-14,"));
}

#[test]
fn json_timestamp_matches_library() {
    use jiff::{civil, tz::TimeZone};
    use rust_zmanim::prelude::*;

    let tmp = TempDir::new().unwrap();
    let out = zmanim(&tmp)
        .args(["shkia"])
        .args(JLM)
        .args(["--date", "2026-07-14", "--format", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    let cli_shkia = parsed[0]["zmanim"]["shkia"].as_str().unwrap();

    // Compute the same value directly through the library.
    let geo = GeoLocation::new(
        31.778,
        35.235,
        0.0,
        TimeZone::get("Asia/Jerusalem").unwrap(),
    )
    .unwrap();
    let czc = ComplexZmanimCalendar::new(geo, civil::date(2026, 7, 14), UseElevation::No);
    let expected = czc.shkia().unwrap();
    let expected_str = expected.strftime("%Y-%m-%dT%H:%M:%S%.f%:z").to_string();

    assert_eq!(cli_shkia, expected_str);
}

/// Each custom spec must reproduce the built-in zman it mirrors.
#[test]
fn custom_offsets_match_their_builtin_equivalents() {
    let pairs = [
        ("tzeis:72min", "tzeis_72_minutes"),
        ("tzeis:90minz", "tzeis_90_minutes_zmanis"),
        // Duration-valued base.
        ("shaah_zmanis_mga:72min", "shaah_zmanis_mga_72_minutes"),
        (
            "sof_zman_shema_mga:16.1deg",
            "sof_zman_shema_mga_16_1_degrees",
        ),
        ("plag_mga:90minz", "plag_mga_90_minutes_zmanis"),
    ];

    for (custom, builtin) in pairs {
        let tmp = TempDir::new().unwrap();
        let run = |name: &str| {
            let out = zmanim(&tmp)
                .args([name])
                .args(JLM)
                .args(["--date", "2026-07-14", "--format", "json"])
                .assert()
                .success()
                .get_output()
                .stdout
                .clone();
            let text = String::from_utf8(out).unwrap();
            let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
            parsed[0]["zmanim"][name].as_str().unwrap().to_string()
        };
        assert_eq!(run(custom), run(builtin), "{custom} != {builtin}");
    }
}

/// A degree value with no built-in equivalent, cross-checked against the
/// library directly.
#[test]
fn custom_degrees_match_the_library() {
    use jiff::{civil, tz::TimeZone};
    use rust_zmanim::prelude::*;

    let tmp = TempDir::new().unwrap();
    let out = zmanim(&tmp)
        .args(["alos:18.5deg"])
        .args(JLM)
        .args(["--date", "2026-07-14", "--format", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    let cli_alos = parsed[0]["zmanim"]["alos:18.5deg"].as_str().unwrap();

    let geo = GeoLocation::new(
        31.778,
        35.235,
        0.0,
        TimeZone::get("Asia/Jerusalem").unwrap(),
    )
    .unwrap();
    let czc = ComplexZmanimCalendar::new(geo, civil::date(2026, 7, 14), UseElevation::No);
    let expected = czc.alos(&ZmanOffset::Degrees(18.5)).unwrap();

    assert_eq!(
        cli_alos,
        expected.strftime("%Y-%m-%dT%H:%M:%S%.f%:z").to_string()
    );
}

/// A negative offset flips the direction: `alos:-72min` is 72 minutes *after*
/// sunrise, and `alos:-5deg` is the sun 5 degrees *above* the horizon (so also
/// after sunrise).
#[test]
fn negative_custom_offsets_flip_direction() {
    use jiff::{SignedDuration, Timestamp};

    let tmp = TempDir::new().unwrap();
    let out = zmanim(&tmp)
        .args([
            "hanetz",
            "alos:-72min",
            "alos:-5deg",
            "shkia",
            "tzeis:-72min",
        ])
        .args(JLM)
        .args(["--date", "2026-07-14", "--format", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    let at =
        |name: &str| -> Timestamp { parsed[0]["zmanim"][name].as_str().unwrap().parse().unwrap() };

    assert_eq!(
        at("alos:-72min"),
        at("hanetz") + SignedDuration::from_mins(72)
    );
    assert_eq!(
        at("tzeis:-72min"),
        at("shkia") - SignedDuration::from_mins(72)
    );
    assert!(at("alos:-5deg") > at("hanetz"));
    assert!(at("alos:-5deg") < at("shkia"));
}

#[test]
fn custom_spec_is_the_csv_header() {
    let tmp = TempDir::new().unwrap();
    let out = zmanim(&tmp)
        .args(["alos:18.5deg", "shkia"])
        .args(JLM)
        .args(["--date", "2026-07-14", "--days", "2", "--format", "csv"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    assert_eq!(text.lines().next().unwrap(), "date,alos:18.5deg,shkia");
}

#[test]
fn bad_custom_spec_errors() {
    let tmp = TempDir::new().unwrap();
    zmanim(&tmp)
        .args(["alos:18.5xyz"])
        .args(JLM)
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("unrecognized offset"));

    let tmp = TempDir::new().unwrap();
    zmanim(&tmp)
        .args(["bogus:72min"])
        .args(JLM)
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("unknown custom zman base"));
}

#[test]
fn list_contains_known_zman() {
    let tmp = TempDir::new().unwrap();
    zmanim(&tmp)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("shkia"))
        .stdout(predicate::str::contains("shaah_zmanis_gra"));
}

#[test]
fn list_filter_and_kind() {
    let tmp = TempDir::new().unwrap();
    let out = zmanim(&tmp)
        .args(["list", "--kind", "duration"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    assert!(text.lines().all(|l| l.contains("shaah_zmanis")));

    // Substring filter, with hyphen->underscore normalization.
    let out = zmanim(&tmp)
        .args(["list", "geonim"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    assert!(!text.is_empty());
    assert!(text.lines().all(|l| l.contains("geonim")));
}

#[test]
fn locations_round_trip() {
    let tmp = TempDir::new().unwrap();

    zmanim(&tmp)
        .args([
            "locations",
            "add",
            "home",
            "--lat",
            "31.778",
            "--lon",
            "35.235",
            "--tz",
            "Asia/Jerusalem",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("added location 'home'"));

    zmanim(&tmp)
        .args(["locations", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("home"));

    // Compute using the saved location (it is the auto-default).
    zmanim(&tmp)
        .args(["shkia", "--date", "2026-07-14"])
        .assert()
        .success()
        .stdout(predicate::str::contains("shkia"));

    zmanim(&tmp)
        .args(["locations", "remove", "home"])
        .assert()
        .success()
        .stdout(predicate::str::contains("removed location 'home'"));
}

#[test]
fn polar_missing_zman_is_null_in_json() {
    let tmp = TempDir::new().unwrap();
    let out = zmanim(&tmp)
        .args(["hanetz"])
        .args(["--lat", "82.5", "--lon", "-62.3", "--tz", "America/Toronto"])
        .args(["--date", "2026-12-21", "--format", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(parsed[0]["zmanim"]["hanetz"].is_null());
}

#[test]
fn polar_missing_zman_is_dash_in_table() {
    let tmp = TempDir::new().unwrap();
    let out = zmanim(&tmp)
        .args(["hanetz"])
        .args(["--lat", "82.5", "--lon", "-62.3", "--tz", "America/Toronto"])
        .args(["--date", "2026-12-21"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    // A bare "contains('-')" would match the date; assert the exact cell.
    let text = String::from_utf8(out).unwrap();
    assert!(
        text.lines().any(|l| l == "hanetz  -"),
        "expected a 'hanetz  -' row, got:\n{text}"
    );
}

#[test]
fn unknown_zman_errors() {
    let tmp = TempDir::new().unwrap();
    zmanim(&tmp)
        .args(["nonsense_zman"])
        .args(JLM)
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("unknown zman"));
}

#[test]
fn no_location_errors() {
    let tmp = TempDir::new().unwrap();
    zmanim(&tmp)
        .args(["shkia"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("no location given"));
}

#[test]
fn bad_date_errors() {
    let tmp = TempDir::new().unwrap();
    zmanim(&tmp)
        .args(["shkia"])
        .args(JLM)
        .args(["--date", "not-a-date"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("invalid date"));
}

#[test]
fn lat_lon_conflicts_with_location() {
    let tmp = TempDir::new().unwrap();
    zmanim(&tmp)
        .args([
            "shkia",
            "--lat",
            "31.0",
            "--lon",
            "35.0",
            "--location",
            "home",
        ])
        .assert()
        .failure()
        .code(2); // clap usage error
}

/// Shells whose clap_complete generator emits possible values for *positional*
/// args. fish/elvish/powershell only do so for named options, so zman names
/// cannot appear in their scripts.
const COMPLETING_SHELLS: [&str; 3] = ["bash", "zsh", "nushell"];

#[test]
fn completions_include_zman_names() {
    // A name that appears nowhere in the help text or the ValueEnum variants,
    // so a hit can only have come from the zman possible-values list.
    let zman = "tzeis_geonim_7_083_degrees";
    for shell in COMPLETING_SHELLS {
        let tmp = TempDir::new().unwrap();
        zmanim(&tmp)
            .args(["completions", shell])
            .assert()
            .success()
            .stdout(predicate::str::contains(zman));
    }
}

/// The 174 names must not leak into `--help`, which is what `hide_possible_values`
/// buys us; the completion test above guards the other side of that tradeoff.
#[test]
fn help_omits_zman_possible_values() {
    let zman = "tzeis_geonim_7_083_degrees";
    for args in [vec!["--help"], vec!["list", "--help"]] {
        let tmp = TempDir::new().unwrap();
        zmanim(&tmp)
            .args(&args)
            .assert()
            .success()
            .stdout(predicate::str::contains(zman).not());
    }
}

/// Ensures the hidden `--config` override path is honored (used by all tests).
#[test]
fn config_override_is_used() {
    let tmp = TempDir::new().unwrap();
    let cfg: PathBuf = tmp.path().join("custom.toml");
    std::fs::write(
        &cfg,
        "default_location = \"z\"\n\n[locations.z]\nlatitude = 31.778\nlongitude = 35.235\ntimezone = \"Asia/Jerusalem\"\n",
    )
    .unwrap();

    Command::cargo_bin("zmanim")
        .unwrap()
        .env_remove("ZMANIM_CONFIG")
        .args(["shkia", "--config"])
        .arg(&cfg)
        .args(["--date", "2026-07-14"])
        .assert()
        .success()
        .stdout(predicate::str::contains("shkia"));
}
