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
