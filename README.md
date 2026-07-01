# Daily Dashboard

[CI](https://github.com/arminv/daily-dashboard/actions/workflows/ci.yml)
[Built with ratatui](https://ratatui.rs)
[License: MIT](LICENSE)

A terminal dashboard you can keep open all day - weather, news, a calendar, a
daily quote, and a built-in dictionary, all in one `ratatui` screen.

![Daily Dashboard Demo](Demo.png)

## Features

- **Greeting & clock** - personalized hello, live time, and IP-based geolocation.
- **Calendar** - current month with today highlighted.
- **Weather** - current conditions plus a 7-day forecast bar chart (Open-Meteo).
- **Daily inspiration** - a fresh quote each day (ZenQuotes).
- **Dictionary** - look up any word and read its definitions inline
  ([Free Dictionary API](https://dictionaryapi.dev)).
- **News feed** - scrollable, categorized headlines; open any article in your
  browser (ok.surf).

No API keys required - every data source is free and public.

## Quick start

```bash
cargo run
```

## Keybindings

| Key                       | Action                                  |
| ------------------------- | --------------------------------------- |
| `q` / `Ctrl-c` / `Ctrl-d` | Quit                                    |
| `Esc`                     | Enter dictionary search mode            |
| `Enter`                   | Search the typed word (in dictionary)   |
| `i` / `↑`                 | Move news selection up                  |
| `j` / `↓`                 | Move news selection down                |
| `Enter`                   | Open selected article in browser (news) |

Keybindings are configurable in `config.json5` (see `.config/config.json5`).

## Data sources

| Widget      | Source                                           | Refresh    |
| ----------- | ------------------------------------------------ | ---------- |
| Greeting    | Public IP + `ipgeolocate` (ip-api.com)           | Once       |
| Weather     | [Open-Meteo](https://open-meteo.com)             | 10 minutes |
| Inspiration | [ZenQuotes](https://zenquotes.io)                | Daily      |
| Dictionary  | [Free Dictionary API](https://dictionaryapi.dev) | On demand  |
| News        | [ok.surf](https://ok.surf)                       | 30 minutes |

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
