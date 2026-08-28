# rust-zmanim-cli

A command-line tool for computing Jewish *zmanim*, powered by the
[rust-zmanim](https://docs.rs/rust-zmanim) library. The installed binary is
named `zmanim`.

## Installation

No `cargo`? Download the right binary from
[releases](https://github.com/YSCohen/rust-zmanim-cli/releases/) and put it in
`~/.local/bin` or your system's equivalent.

With [`cargo-binstall`](https://github.com/cargo-bins/cargo-binstall), that same
binary gets downloaded and placed for you:

```sh
cargo binstall rust-zmanim-cli
```

Or build it from source:

```sh
cargo install rust-zmanim-cli
```

## Usage

Compute zmanim by name for a location and date:

```sh
zmanim shkia tzeis_72_minutes --lat 31.778 --lon 35.235 --tz Asia/Jerusalem
```

Zman names match the library's method names; hyphens and underscores are
interchangeable (`tzeis-72-minutes` == `tzeis_72_minutes`). Run `zmanim list`
to see them all, optionally narrowed by a substring (e.g. `zmanim list geonim`) or
by kind (e.g. `zmanim list --kind duration` for *shaah zmanis* calculations).

With no zman names given, a default set is used: *hanetz*, *sof zman shema*
GRA, *chatzos hayom*, *mincha gedola* GRA, *shkia*, and *tzeis geonim* 8.5°.
Override it with the `zmanim` key in the config file.

### Custom offsets

A *zman* that isn't in the built-in list can be spelled out directly as
`<base>:<value><unit>`:

```sh
zmanim alos:18.5deg shaah_zmanis_mga:80min tzeis:100minz
```

| Unit | Meaning |
| --- | --- |
| `deg` | Degrees below the horizon |
| `min` | Fixed clock minutes before/after sunrise/sunset |
| `minz` | Minutes *zmaniyos*, each 1/60 of the GRA (sunrise-to-sunset) *shaah zmanis* |

A negative value flips the direction: `alos:-72min` is 72 minutes *after*
sunrise, and `alos:-5deg` puts the sun 5° *above* the horizon rather than below
it. Degrees must be between -90 and 90.

The base is one of the ten *zman* families that take an offset: `alos`,
`tzeis`, `shaah_zmanis_mga`, `sof_zman_shema_mga`, `sof_zman_tefila_mga`,
`sof_zman_biur_chametz_mga`, `mincha_gedola_mga`, `samuch_lemincha_ketana_mga`,
`mincha_ketana_mga`, and `plag_mga`. For the `*_mga` families the offset
defines both ends of the day, exactly as it does for the built-in names — so
`tzeis:72min` is the same *zman* as `tzeis_72_minutes`, and
`sof_zman_shema_mga:16.1deg` the same as `sof_zman_shema_mga_16_1_degrees`.

The spec is used verbatim as the column header, JSON key, or CSV field name.
Custom offsets work in the config file's `zmanim` list too, but shell
completion only knows the built-in names.

### Location

Provide raw coordinates, or save a named location and reuse it:

```sh
zmanim locations add home --lat 31.778 --lon 35.235 --elevation 754 --tz Asia/Jerusalem
zmanim shkia --location home        # or just `zmanim shkia` once a default is set
```

The first location you add becomes the default. Manage the rest with
`zmanim locations list | remove | set-default`.

`--tz`, `--elevation`, and `--use-elevation` override the corresponding saved
values for a single run, so you can reuse a location with one field changed:

```sh
zmanim shkia --location home --elevation 0
```

### Elevation

`--elevation` (meters, default 0) only matters if you also opt in to using it.
`--use-elevation` controls that:

| Mode | Effect |
| --- | --- |
| `no` (default) | Ignore elevation entirely |
| `hanetz-shkia` | Use it for sunrise and sunset only |
| `all` | Use it for every *zman* |

This visibly moves times — at 754 m in Jerusalem, `--use-elevation all` puts
*shkia* about 4½ minutes later than `no`. Set a per-location policy with
`use_elevation` in the config so you don't have to pass the flag each time.

### Dates

```sh
zmanim shkia                                   # today
zmanim shkia --date 2026-07-14
zmanim shkia --date tomorrow
zmanim shkia --date 2026-07-14..2026-07-20     # inclusive range
zmanim shkia --date 2026-07-14 --days 7        # 7 consecutive days
```

### Output

`--format` takes `table` (the default), `csv`, or `json`. Times display to the
minute; `--precision m|s|ms` and `--round nearest|down|up` change that.

`--time-style` picks how times are written: `auto` (the default — clock times
for table and CSV, full-precision RFC-3339 for JSON), or `human`/`iso` to force
one of them in any format.

Missing zmanim — such as a *hanetz* that never happens above the Arctic Circle —
render as `-` in table, an empty field in CSV, and `null` in JSON.

## Command reference

<details>
<summary><code>zmanim --help</code></summary>

```console
$ zmanim --help
A command-line tool for computing Jewish zmanim, powered by rust-zmanim

Usage: zmanim [OPTIONS] [ZMAN]... [COMMAND]

Commands:
  list         List available zman names (optionally filtered by a substring)
  locations    Manage saved locations
  completions  Generate a shell completion script
  help         Print this message or the help of the given subcommand(s)

Arguments:
  [ZMAN]...
          Zmanim to compute, by name (see `zmanim list`). Hyphens and underscores are interchangeable. If omitted, a default set from config is used.
          
          A custom offset can be given as `<base>:<value><unit>`, e.g. `alos:18.5deg`, `shaah_zmanis_mga:80min`, `tzeis:100minz`. Units are `deg` (degrees below the horizon), `min` (clock minutes) and `minz` (minutes zmaniyos). A negative value flips the direction, so `alos:-72min` is 72 minutes after sunrise. Bases: alos, tzeis, shaah_zmanis_mga, sof_zman_shema_mga, sof_zman_tefila_mga, sof_zman_biur_chametz_mga, mincha_gedola_mga, samuch_lemincha_ketana_mga, mincha_ketana_mga, plag_mga.

Options:
  -L, --location <NAME>
          Saved location name (see `zmanim locations`)

          [env: ZMANIM_LOCATION=]

      --lat <LAT>
          Latitude in decimal degrees (requires --lon)

      --lon <LON>
          Longitude in decimal degrees (requires --lat)

      --elevation <METERS>
          Elevation in meters (default 0)

      --tz <TZ>
          IANA timezone (e.g. Asia/Jerusalem). Defaults to the location's saved timezone, or the system timezone for raw coordinates

          [env: ZMANIM_TZ=]

      --date <DATE>
          Date, `today`, `tomorrow`, or an inclusive range `START..END`

      --days <N>
          Number of consecutive days starting from --date (or today). Cannot be combined with a `START..END` range

      --use-elevation <MODE>
          Elevation policy for the calculations

          Possible values:
          - no:           Never use elevation
          - hanetz-shkia: Use elevation only for sunrise/sunset
          - all:          Use elevation for all zmanim

          [env: ZMANIM_USE_ELEVATION=]

      --format <FORMAT>
          Output format

          Possible values:
          - table: Aligned text table (or a vertical list for a single date)
          - csv:   Comma-separated values
          - json:  JSON array of per-date objects

          [env: ZMANIM_FORMAT=]

      --precision <PRECISION>
          Display precision for human-readable times/durations

          Possible values:
          - m:  Minutes
          - s:  Seconds
          - ms: Milliseconds

          [env: ZMANIM_PRECISION=]

      --round <ROUND>
          Rounding mode for human-readable times/durations

          Possible values:
          - nearest: Round to the nearest unit
          - down:    Round toward earlier times (floor)
          - up:      Round toward later times (ceil)

          [env: ZMANIM_ROUND=]

      --time-style <STYLE>
          Time rendering: `auto` (human for table/csv, ISO for json), or force `human`/`iso` in any format

          Possible values:
          - auto:  Human clock times for table/csv, full-precision ISO for json
          - human: Human-readable clock times/durations (honors precision and round)
          - iso:   Full-precision ISO 8601 (RFC-3339 times, `PT..` durations)

          [env: ZMANIM_TIME_STYLE=]

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version

EXAMPLES:
  # Compute zmanim for raw coordinates, today
  zmanim shkia tzeis_72_minutes --lat 31.778 --lon 35.235 --tz Asia/Jerusalem

  # Save a location
  zmanim locations add home --lat 31.778 --lon 35.235 --tz Asia/Jerusalem

  # Use a saved location and a date range
  zmanim sof_zman_shema_gra shkia --location home --date 2026-07-14..2026-07-20

  # Default zman set (from config), as JSON, next 7 days
  zmanim --location home --days 7 --format json

  # Search zman names
  zmanim list geonim

  # Custom offsets: 18.5 degrees for alos, 70 clock minutes for tzeis
  zmanim alos:18.5deg tzeis:70min --location home
```

</details>

<details>
<summary><code>zmanim list --help</code></summary>

```console
$ zmanim list --help
List available zman names (optionally filtered by a substring)

Usage: zmanim list [OPTIONS] [FILTER]

Arguments:
  [FILTER]
          Only show names containing this substring

Options:
      --kind <KIND>
          Only show zmanim of this kind

          Possible values:
          - time:     Instant-in-time zmanim
          - duration: Duration zmanim (*shaah zmanis*)

  -h, --help
          Print help (see a summary with '-h')
```

</details>

<details>
<summary><code>zmanim locations --help</code></summary>

```console
$ zmanim locations --help
Manage saved locations

Usage: zmanim locations [COMMAND]

Commands:
  list         List saved locations (the default when no action is given)
  add          Add or update a saved location
  remove       Remove a saved location
  set-default  Set the default location
  help         Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

</details>

<details>
<summary><code>zmanim completions --help</code></summary>

```console
$ zmanim completions --help
Generate a shell completion script

Usage: zmanim completions <SHELL>

Arguments:
  <SHELL>  Shell to generate completions for [possible values: bash, elvish,
           fish, nushell, powershell, zsh]

Options:
  -h, --help  Print help
```

</details>

## Configuration

Config lives at `~/.config/zmanim/config.toml`. Settings resolve in the order
**flags > `ZMANIM_*` environment variables > config file > built-in defaults**.

```toml
default_location = "home"
zmanim = ["hanetz", "sof_zman_shema_gra", "shkia", "tzeis_72_minutes"]
format = "table"
precision = "m"
round = "nearest"
time_style = "auto"

[locations.home]
latitude = 31.778
longitude = 35.235
elevation = 754.0               # optional, meters, default 0
timezone = "Asia/Jerusalem"
use_elevation = "hanetz-shkia"  # optional: no | hanetz-shkia | all
```

Unknown keys are rejected rather than ignored, so a typo fails loudly.

### Environment variables

Each takes the same values as the flag it shadows:

| Variable | Flag |
| --- | --- |
| `ZMANIM_LOCATION` | `--location` |
| `ZMANIM_TZ` | `--tz` |
| `ZMANIM_USE_ELEVATION` | `--use-elevation` |
| `ZMANIM_FORMAT` | `--format` |
| `ZMANIM_PRECISION` | `--precision` |
| `ZMANIM_ROUND` | `--round` |
| `ZMANIM_TIME_STYLE` | `--time-style` |

`--lat`, `--lon`, `--elevation`, `--date`, and `--days` have no environment
variable — pass them as flags.

## Shell completions

`zmanim completions <SHELL>` writes a completion script to stdout.

On bash, zsh, and nushell these complete *zman* names too. fish, elvish, and
PowerShell get flags and subcommands only: `clap_complete`'s generators for
those don't emit possible values for positional arguments, so *zman* names
would need a more complicated setup that I didn't want to add, for now at least.

<details>
<summary>Setup, per shell</summary>

Add the relevant lines to that shell's `...rc` or config file.

**bash**:

```sh
source <(zmanim completions bash)
```

**zsh**:

```sh
source <(zmanim completions zsh)
```

**fish**:

```sh
zmanim completions fish > ~/.config/fish/completions/zmanim.fish
```

**nushell**:  
(note that if both lines live in the same file, the `source` runs before the
`save` that generates the file)

```nu
zmanim completions nushell | save --force $"($nu.cache-dir)/zmanim-completions.nu"
source $"($nu.cache-dir)/zmanim-completions.nu"
```

**elvish**:

```elv
eval (zmanim completions elvish | slurp)
```

**PowerShell**:

```powershell
zmanim completions powershell | Out-String | Invoke-Expression
```

</details>

## A note on AI assistance

**Almost all of this CLI's code was written by Claude Code.** I wrote the
specification and made the design calls. I also reviewed, tested,
and corrected the result. However, as the code itself was mostly generated,
there may be some of the odd kind of mistakes or code style which only LLMs
make. Bear this in mind if you want to contribute, or just read the code.

Of course, I would be happy to field any bug reports, feature requests, or even
grumpy complaints. Thanks in advance!
