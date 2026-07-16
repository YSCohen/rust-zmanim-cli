# rust-zmanim-cli

A command-line tool for computing Jewish *zmanim*, powered by the
[rust-zmanim](https://docs.rs/rust-zmanim) library. The installed binary is named `zmanim`.

## Install

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
to see them all (optionally filtered, e.g. `zmanim list geonim`).

### Location

Provide raw coordinates, or save a named location and reuse it:

```sh
zmanim locations add home --lat 31.778 --lon 35.235 --elevation 754 --tz Asia/Jerusalem
zmanim shkia --location home        # or just `zmanim shkia` once a default is set
```

The first location you add becomes the default. Manage the rest with
`zmanim locations list | remove | set-default`.

### Dates

```sh
zmanim shkia                                   # today
zmanim shkia --date 2026-07-14
zmanim shkia --date tomorrow
zmanim shkia --date 2026-07-14..2026-07-20     # inclusive range
zmanim shkia --date 2026-07-14 --days 7        # 7 consecutive days
```

### Output

`--format table` (default), `csv`, or `json`. Times display in minutes by
default; control this with `--precision m|s|ms` and `--round nearest|down|up`.
JSON emits full-precision RFC-3339 timestamps by default (override with
`--time-style human|iso`). Missing zmanim (e.g. in polar regions) render as
`-` (table), an empty field (CSV), or `null` (JSON).

## Configuration

Config lives at `~/.config/zmanim/config.toml`. Settings resolve in the order
**flags > `ZMANIM_*` environment variables > config file > built-in defaults**.

```toml
default_location = "home"
zmanim = ["hanetz", "sof_zman_shema_gra", "shkia", "tzeis_72_minutes"]
format = "table"
precision = "m"
round = "nearest"

[locations.home]
latitude = 31.778
longitude = 35.235
elevation = 754.0               # optional, meters, default 0
timezone = "Asia/Jerusalem"
use_elevation = "hanetz-shkia"  # optional: no | hanetz-shkia | all
```


<details>
<summary>Shell completions</summary>

**bash** — add to `~/.bashrc`:

```sh
source <(zmanim completions bash)
```

**zsh** — add to `~/.zshrc`:

```sh
source <(zmanim completions zsh)
```

**fish** — completions are auto-loaded from this directory, so a one-time generation is enough:

```sh
zmanim completions fish > ~/.config/fish/completions/zmanim.fish
```

**nushell** — add to `config.nu`:

```nu
zmanim completions nushell | save --force $"($nu.cache-dir)/zmanim-completions.nu"
source $"($nu.cache-dir)/zmanim-completions.nu"
```

**elvish** — add to `rc.elv`:

```elv
eval (zmanim completions elvish | slurp)
```

**PowerShell** — add to `$PROFILE`:

```powershell
zmanim completions powershell | Out-String | Invoke-Expression
```

</details>
