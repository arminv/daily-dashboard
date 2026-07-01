# AGENTS.md

This file provides guidance to AI agents when working with code in this repository.

## Project Overview

Daily Dashboard is a Terminal User Interface (TUI) application written in Rust that shows a greeting/location, calendar, weather, daily inspirational quote, an interactive dictionary, and a news feed in a single dashboard. It uses `ratatui` for the TUI framework and follows an async, component-based architecture.

## Essential Commands

### Build & Run

```bash
# Build the project
cargo build

# Run with default settings (4 ticks/sec, 60 frames/sec)
cargo run

# Run with custom tick/frame rates
cargo run -- --tick-rate 2.0 --frame-rate 30.0

# Show CLI help; --version also prints the resolved config & data directories
cargo run -- --help
cargo run -- --version
```

### Quality gates (match CI exactly)

```bash
cargo test --locked --all-features --workspace
cargo fmt --all --check
cargo clippy --all-targets --all-features --workspace -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items --all-features --workspace --examples

# Run the live-network tests that are #[ignore]d by default
cargo test -- --ignored
```

### Development

- Main entry point: `src/main.rs`
- Default config (embedded): `.config/config.json5` (JSON5 with comments)
- User config directory: platform-specific via the `directories` crate (override with `DAILY_DASHBOARD_CONFIG`)
- Logs: `logs.log` file for debugging

## Source Map

| File                  | Responsibility                                                                                                                  |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| `src/main.rs`         | Entry point: error/logging init, CLI parse, run `App`. Declares all top-level modules.                                          |
| `src/app.rs`          | `App`: owns the event loop, config, the single `Dashboard`, and the action channel. Defines `Mode` and `LoadingStatus`.         |
| `src/tui.rs`          | Wraps ratatui/crossterm; runs a background event loop (tick, render, key, mouse, resize) over an mpsc channel.                  |
| `src/action.rs`       | The `Action` enum (Tick, Render, Resize, Quit, Suspend, Resume, Error, …).                                                      |
| `src/dashboard.rs`    | The only top-level `Component` registered in `App`. Owns and lays out all widgets.                                              |
| `src/components.rs`   | The `Component` trait and the widget submodule declarations.                                                                    |
| `src/components/*.rs` | The widgets: `calendar`, `greeting`, `weather`, `inspiration`, `dictionary`, `news`, `fps` (disabled).                          |
| `src/http.rs`         | Shared `reqwest::Client` + `get_json` / `get_text` helpers used by every fetch.                                                 |
| `src/theme.rs`        | Centralized border/title/color styles (`panel_block`, `panel_block_colored`, `frame_block`, `ACCENT`/`LOADING`/`ERROR`/`HINT`). |
| `src/config.rs`       | Layered config loading + keybinding/style parsing.                                                                              |
| `src/cli.rs`          | `clap` CLI definition (`--tick-rate`, `--frame-rate`).                                                                          |
| `src/errors.rs`       | `color_eyre` hook + panic handler.                                                                                              |
| `src/logging.rs`      | `tracing` subscriber setup.                                                                                                     |
| `src/tests/*.rs`      | Unit/render tests, included per-widget via `#[cfg(test)] #[path = "../tests/<name>.rs"] mod tests;`.                            |

## Architecture Overview

### Application Flow

1. `main.rs` initializes error handling and logging, parses CLI args, and runs `App`.
2. `App` (`src/app.rs`) owns the event loop, config, and the single `Dashboard` (held directly, not behind a `Vec<Box<dyn Component>>`). It also builds the shared HTTP client via `crate::http::shared_client()` and passes it down through `Dashboard::new(client)`.
3. `Tui` (`src/tui.rs`) wraps ratatui/crossterm and runs a background event loop (tick, render, keyboard, resize) feeding events into an mpsc channel.
4. Actions flow through `tokio::sync::mpsc` unbounded channels between the TUI layer and components.

### Component System

Each widget implements the `Component` trait defined in `src/components.rs`:

- `draw()` — render the component (required)
- `update()` — react to `Action`s (e.g. spawn fetches on `Action::Tick`)
- `handle_events()` — process input events; returning `Some(Action)` signals that the event was consumed
- `register_action_handler()` / `register_config_handler()` / `init()` — lifecycle hooks

`Dashboard` is the only component `App` dispatches to directly. It overrides `register_action_handler`, `register_config_handler`, and `init` to **forward** each to its typed children, and overrides `handle_events` / `update` / `draw` to iterate them.

### Dashboard Composite

`src/dashboard.rs` composes and lays out all widgets:

```
┌──────────────────────────────────────────────────────────────┐
│  Top 40%                                                      │
│  ┌───────────────────┬─────────────────┬────────────────────┐ │
│  │ Calendar          │ Weather         │ Dictionary         │ │
│  │ (greeting left,   │ (current cond.  │ (search input +    │ │
│  │  grid top-right)  │  + 7-day fc)    │  definitions)      │ │
│  ├───────────────────┤                 │                    │ │
│  │ Daily Inspiration │                 │                    │ │
│  └───────────────────┴─────────────────┴────────────────────┘ │
├──────────────────────────────────────────────────────────────┤
│  Bottom 60%: News feed table                                  │
└──────────────────────────────────────────────────────────────┘
```

Layout details (`Dashboard::draw`):

- Vertical split: top 40% / bottom 60% (`Flex::SpaceBetween`, 1-row spacing).
- Top row: three horizontal columns (`Min(30)` each).
- **Left column** is a single framed panel (`theme::frame_block("📅 Calendar")`) owned by the Dashboard. Inside, the greeting and month grid are laid out responsively: when the panel is wide enough (inner width ≥ `GREETING_MIN_WIDTH + MONTHLY_WIDTH` = 49), `Greeting` sits on the left and the `Calendar` grid goes at the top-right; on narrower terminals they stack — greeting on top, grid below (right-aligned) — so the datetime line isn't clipped. The grid itself is right-aligned within whatever area it receives (`Calendar::draw`). Below the panel sits `Inspiration` (`Length(6)`).
- Middle column: `Weather`. Right column: `Dictionary` (occupies the full top-right column so the definition renders below its input). Bottom: `News`.

Greeting and Calendar no longer coordinate hardcoded offsets — the Dashboard owns the frame and hands each widget a `Layout`-derived sub-`Rect`.

### Widgets

- **Greeting** (`src/components/greeting.rs`) — username greeting, live clock/datetime, and IP-based location. Source of the shared location state consumed by Weather.
- **Calendar** (`src/components/calendar.rs`) — monthly calendar with today highlighted; grid placed at the top-right of the area the Dashboard allocates.
- **Weather** (`src/components/weather.rs`) — current conditions and a 7-day forecast bar chart via Open-Meteo. Reads location from the Greeting's shared state (does **not** create its own Greeting).
- **Inspiration** (`src/components/inspiration.rs`) — daily inspirational quote via ZenQuotes.
- **Dictionary** (`src/components/dictionary.rs`) — interactive word lookup via the Free Dictionary API; `ratatui-textarea` input with inline definition rendering.
- **News** (`src/components/news.rs`) — scrollable, categorized news table with keyboard navigation and browser links.
- **FPS** (`src/components/fps.rs`) — performance counter (disabled / dead code).

### Event System

- Actions are defined in `src/action.rs` (Tick, Render, Resize, Quit, Suspend, Resume, Error, …).
- Components communicate via `tokio::sync::mpsc` unbounded channels; the main loop in `src/app.rs` dispatches actions to `Dashboard`, which fans them out to children.
- Configurable keybindings in `config.json5` map keys to actions per `Mode` (only `Home` exists).
- **Event propagation:** `Dashboard::handle_events` iterates children and **stops** as soon as one returns `Some(action)` (used by the Dictionary while editing, so keypresses don't also reach News).
- **Two key paths:** global keys (`q`, `Ctrl-c`, `Ctrl-d`, `Ctrl-z`) go through the config keymap → `Action`. Widget-specific keys (News navigation, Dictionary input) are handled directly in each widget's `handle_events` and bypass the keymap.

### State Management

- Widget state uses `Arc<RwLock<T>>` (`std::sync`) for thread-safe shared state.
- `LoadingStatus` enum (`src/app.rs`) tracks async states: `NotStarted`, `Loading`, `Loaded`, `Error(String)`. Error strings use `Display` formatting (`{e}`), not `{e:?}`.
- Async data fetching uses `tokio::spawn` inside `update()` on `Action::Tick` (or on submit for the Dictionary). The `TextArea` is not `Send`, so the Dictionary keeps it outside the `Arc<RwLock<DictionaryData>>` it shares with its spawn task.
- **Weather depends on location data from the Greeting component's shared state** — `Dashboard` constructs `Greeting` first and passes `greeting.state.clone()` into `Weather::new(...)`, so there is exactly one location fetch on startup.

### HTTP & Data Fetching

- `src/http.rs` builds one shared `reqwest::Client` (10s timeout, `daily-dashboard/<version>` user-agent, pooled connections). Cloning a `Client` is cheap, so the single client is cloned into each fetching widget instead of being rebuilt per request.
- `http::get_json(url)` and `http::get_text(url)` handle the GET → `error_for_status` → body decode → JSON parse ladder, returning `color_eyre::Result`. Every widget fetch (greeting, weather, news, inspiration, dictionary) goes through these helpers.
- IP **geolocation** still uses the `ipgeolocate` crate (`ip-api.com`) directly, separate from the `http` helpers.

### Theming

- `src/theme.rs` centralizes the visual language so widgets don't hardcode styles:
  - `panel_block(title)` / `panel_block_colored(title, color)` — bordered block drawn **before** content (content renders into `block.inner(area)`); uses `.style()` so the accent reaches empty cells.
  - `frame_block(title)` — bordered block drawn **after** content (the shared Calendar panel); uses only `.border_style()` / `.title_style()` (no `.style()`) so it doesn't recolor already-drawn cells.
  - Color constants: `ACCENT` (Cyan, borders/titles), `LOADING` (Yellow), `ERROR` (Red), `HINT` (DarkGray).

### Refresh Intervals

- **Greeting / Location**: fetched once on startup.
- **Weather**: every 10 minutes (after location is loaded).
- **News**: every 30 minutes.
- **Inspiration**: once on the first tick (daily quote).
- **Dictionary**: on demand, when the user submits a word.

## Key Dependencies

- **ratatui** 0.30.1 — TUI framework (`widget-calendar` feature for the month grid)
- **ratatui-textarea** 0.9.2 — editable text input for the Dictionary
- **crossterm** 0.29.0 — terminal input/output (with `event-stream`)
- **tokio** 1.40.0 — async runtime (full features)
- **reqwest** 0.13.4 — HTTP client (`json` feature) for all API calls
- **serde** / **serde_json** / **json5** — (de)serialization + JSON5 config
- **clap** 4.5.20 — CLI argument parsing
- **config** 0.15.23 — layered configuration with JSON5 support
- **color-eyre** 0.6.3 — error reporting (the app standardizes on `color_eyre::Result`)
- **tracing** 0.1.40 / **tracing-subscriber** — structured logging
- **ipgeolocate** 0.3 — IP-based geolocation via ip-api.com
- **chrono** 0.4 / **time** 0.3.41 — date/time handling
- **open** 5.0 — open URLs in the default browser (News)
- **whoami** 2.1.2 — current username (Greeting)

## Configuration System

Configuration is layered:

1. Embedded defaults from `.config/config.json5` (compiled into the binary).
2. User overrides from the config directory (`config.json5`, `config.json`, `config.yaml`, `config.toml`, or `config.ini`).

Supports:

- Keybindings per mode (`Home`).
- Style configuration with color parsing.
- Environment variable overrides (`DAILY_DASHBOARD_CONFIG`, `DAILY_DASHBOARD_DATA`).

Default keybindings (config-driven, global):

- `q`, `Ctrl-d`, `Ctrl-c` — Quit
- `Ctrl-z` — Suspend (returns to shell)

Widget keys (handled directly in each widget, **not** via the config keymap):

- **News**: `i` / `Up` — move selection up · `j` / `Down` — move selection down · `Enter` — open the selected article in the default browser.
- **Dictionary**: `Esc` — enter editing mode (or leave it) · type to edit · `Enter` — look up the word. While editing, the Dictionary consumes all keypresses so News doesn't react.

## Data Sources & APIs

| Widget      | Source                                                                   | Notes                                                                                 |
| ----------- | ------------------------------------------------------------------------ | ------------------------------------------------------------------------------------- |
| Weather     | [Open-Meteo](https://open-meteo.com/)                                    | No API key. Current conditions + 7-day forecast.                                      |
| Location    | Public IP (ipify / ifconfig.me / icanhazip) → `ipgeolocate` (ip-api.com) | IP tried from multiple endpoints; geolocation via the `ipgeolocate` crate.            |
| Inspiration | [ZenQuotes](https://zenquotes.io/api/today)                              | Daily quote (`q` text, `a` author).                                                   |
| Dictionary  | [Free Dictionary API](https://api.dictionaryapi.dev/api/v2/entries/en)   | Word → phonetic + meanings/definitions. 404 on unknown words is surfaced as an error. |
| News        | [ok.surf](https://ok.surf/api/v1/cors/news-feed)                         | Business, Technology, Sports, Politics, Health, Entertainment.                        |

## Testing Notes

- Tests live in `src/tests/*.rs` and are attached per-module with `#[cfg(test)] #[path = "../tests/<name>.rs"] mod tests;`, giving them access to private items via `use super::*;`.
- **Pure parse functions** are extracted out of the fetch paths and unit-tested:
  - `news::parse_articles` — category ordering, missing-field skipping, per-category cap, unknown/empty categories.
  - `weather::parse_daily_forecast` — weekday/temp extraction, missing `daily`, bad dates, partial arrays.
  - `dictionary::parse_entry` and `dictionary::build_definition_text` — word/phonetic/meaning extraction, phonetics-array fallback, empty-meaning skipping, rendered text.
- **Render snapshot tests** use `ratatui::backend::TestBackend` + `Terminal` to draw a widget into a buffer and assert on the visible text (see `src/tests/inspiration.rs`).
- **Live-network tests** are marked `#[ignore]` so the default suite is hermetic; run them with `cargo test -- --ignored`.
- Lint and doc gates are strict: `clippy --all-targets -- -D warnings` and `cargo doc --document-private-items` with `RUSTDOCFLAGS=-D warnings`.

## CI/CD

- **CI** (`.github/workflows/ci.yml`): on push to `main` / PRs, runs tests, `rustfmt --check`, clippy (`-D warnings`), and docs (`-D warnings`) on nightly.
- **CD** (`.github/workflows/cd.yml`): cross-platform release builds and `cargo publish` on version tags.
