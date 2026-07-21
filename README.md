# Daily Dashboard

[![CI](https://github.com/arminv/daily-dashboard/actions/workflows/ci.yml/badge.svg)](https://github.com/arminv/daily-dashboard/actions/workflows/ci.yml)
[![Built with ratatui](https://img.shields.io/badge/built%20with-ratatui-blue)](https://ratatui.rs)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

A terminal dashboard you can keep open all day - weather, news, a calendar, a
daily picture, a daily quote, a built-in dictionary, and Wikipedia search, all
in one `ratatui` screen.

![Daily Dashboard Demo](Demo.png)

## Features

- **Greeting & clock** - personalized hello, live time, and IP-based geolocation.
- **Calendar** - current month with today highlighted.
- **Weather** - current conditions plus a 7-day forecast bar chart (Open-Meteo).
- **Daily Picture** - a random photo from [Lorem Picsum](https://picsum.photos),
  rendered inline via `ratatui-image` (kitty/iTerm2/sixel, with a
  unicode-halfblocks fallback). Press `Shift+N` to fetch a new image.
- **Daily inspiration** - a fresh quote each day (ZenQuotes).
- **Dictionary** - look up any word and read its definitions inline
  ([Free Dictionary API](https://dictionaryapi.dev)).
- **Wikipedia** - search English Wikipedia, browse results with lead extracts,
  and open articles in your browser ([MediaWiki API](https://www.mediawiki.org/wiki/API:Main_page)).
- **News feed** - scrollable, categorized headlines; open any article in your
  browser (ok.surf).

No API keys required - every data source is free and public, including the
daily picture (served by [Lorem Picsum](https://picsum.photos), which needs no key
and has no meaningful rate limit) and Wikipedia.

The daily picture is rendered via `ratatui-image`, which auto-detects your
terminal's graphics protocol (kitty / iTerm2 / sixel) and falls back to unicode
halfblocks.

If a terminal shows `[?]` boxes instead of the photo, auto-detection picked a protocol
it can't actually render (common inside `tmux`); set
`DAILY_DASHBOARD_IMAGE_PROTOCOL=halfblocks` (works everywhere) or force
`kitty` / `sixel` / `iterm2` / `auto`.

## Quick start

```bash
cargo run
```

## Keybindings

**Global** (configurable in `config.json5`; see `.config/config.json5`):

| Key                       | Action                    |
| ------------------------- | ------------------------- |
| `q` / `Ctrl-c` / `Ctrl-d` | Quit                      |
| `Ctrl-z`                  | Suspend (return to shell) |

**Dictionary**

| Key     | Action                                 |
| ------- | -------------------------------------- |
| `Esc`   | Enter / leave search mode              |
| `Enter` | Look up the typed word (while editing) |

**Wikipedia**

| Key     | Action                                                             |
| ------- | ------------------------------------------------------------------ |
| `/`     | Enter search mode                                                  |
| `Esc`   | Leave search mode                                                  |
| `↑`/`↓` | Move result selection (while editing; list scrolls with selection) |
| `Enter` | Search, or open the selected article if the query is unchanged     |

**News**

| Key       | Action                |
| --------- | --------------------- |
| `i` / `↑` | Move selection up     |
| `j` / `↓` | Move selection down   |
| `Enter`   | Open selected article |

**Daily Picture**

| Key       | Action                    |
| --------- | ------------------------- |
| `Shift+N` | Fetch a new daily picture |

While Dictionary or Wikipedia is editing, that widget consumes all keypresses so
siblings (e.g. News) do not also react.

## Data sources

| Widget        | Source                                                                   | Refresh                  |
| ------------- | ------------------------------------------------------------------------ | ------------------------ |
| Greeting      | Public IP + [ip-api.com](https://ip-api.com)                             | Once                     |
| Weather       | [Open-Meteo](https://open-meteo.com)                                     | 10 minutes               |
| Inspiration   | [ZenQuotes](https://zenquotes.io)                                        | Daily                    |
| Dictionary    | [Free Dictionary API](https://dictionaryapi.dev)                         | On demand                |
| Wikipedia     | [MediaWiki search](https://www.mediawiki.org/wiki/API:Search) + extracts | On demand (2 req/search) |
| News          | [ok.surf](https://ok.surf)                                               | 30 minutes               |
| Daily Picture | [Lorem Picsum](https://picsum.photos)                                    | Startup + on-demand      |

## Built with

[Rust](https://www.rust-lang.org) · [ratatui](https://ratatui.rs) ·
[crossterm](https://docs.rs/crossterm) · [tokio](https://tokio.rs) ·
[reqwest](https://docs.rs/reqwest)

## Development

```bash
cargo build       # build
cargo test        # run tests
cargo clippy      # lint
cargo fmt         # format
```

## License

[MIT](LICENSE) © Armin Varshokar
