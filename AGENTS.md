# AGENTS.md

This file provides guidance to AI agents when working with code in this repository.

## Project Overview

Daily Dashboard is a Terminal User Interface (TUI) application written in Rust that shows a greeting/location, calendar, weather, a daily picture, a daily inspirational quote, an interactive dictionary, Wikipedia search, and a news feed in a single dashboard. It uses `ratatui` for the TUI framework and follows an async, component-based architecture.

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
- Logs: `logs.log` in the process cwd (gitignored; typically the repo root under `cargo run`)

## Source Map

| File                  | Responsibility                                                                                                                       |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| `src/main.rs`         | Entry point: error/logging init, CLI parse, run `App`. Declares all top-level modules.                                               |
| `src/app.rs`          | `App`: owns the event loop, config, the single `Dashboard`, and the action channel. Defines `Mode` and `LoadingStatus`.              |
| `src/tui.rs`          | Wraps ratatui/crossterm; runs a background event loop (tick, render, key, mouse, resize) over an mpsc channel.                       |
| `src/action.rs`       | The `Action` enum (Tick, Render, Resize, Quit, Suspend, Resume, Error, …).                                                           |
| `src/dashboard.rs`    | The only top-level `Component` registered in `App`. Owns and lays out all widgets.                                                   |
| `src/components.rs`   | The `Component` trait and the widget submodule declarations.                                                                         |
| `src/components/*.rs` | The widgets: `calendar`, `greeting`, `weather`, `picture_frame`, `wikipedia`, `inspiration`, `dictionary`, `news`, `fps` (disabled). |
| `src/http.rs`         | Shared `reqwest::Client` + `get_json` / `get_text` / `get_bytes_redirected` helpers used by every fetch.                             |
| `src/theme.rs`        | Centralized border/title/color styles (`panel_block`, `panel_block_colored`, `frame_block`, `ACCENT`/`LOADING`/`ERROR`/`HINT`).      |
| `src/config.rs`       | Layered config loading + keybinding/style parsing.                                                                                   |
| `src/cli.rs`          | `clap` CLI definition (`--tick-rate`, `--frame-rate`).                                                                               |
| `src/errors.rs`       | `color_eyre` hook + panic handler.                                                                                                   |
| `src/logging.rs`      | `tracing` subscriber setup.                                                                                                          |
| `src/tests/*.rs`      | Unit/render tests, included per-widget via `#[cfg(test)] #[path = "../tests/<name>.rs"] mod tests;`.                                 |

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
┌──────────────────────────────────────────────────────────────────────────┐
│  Top 40%                                                                  │
│  ┌──────────────┬────────────┬──────────────┬──────────────────────────┐ │
│  │ Calendar     │ Weather    │ Daily Picture│ Dictionary               │ │
│  │ (greeting +  │ (current   │ (Lorem Picsum│ (search + definitions)   │ │
│  │  grid)       │  + 7-day)  │  photo)      │                          │ │
│  ├──────────────┤            │              │                          │ │
│  │ Inspiration  │            │              │                          │ │
│  └──────────────┴────────────┴──────────────┴──────────────────────────┘ │
├──────────────────────────────────────────────────────────────────────────┤
│  Bottom 60%                                                               │
│  ┌──────────────────────┬───────────────────────────────────────────────┐ │
│  │ Wikipedia (1/3)       │ News feed table (2/3)                         │ │
│  │ (search + results +   │ (scrollable, categorized)                     │ │
│  │  extract preview)     │                                               │ │
│  └──────────────────────┴───────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────────────┘
```

Layout details (`Dashboard::draw`):

- Vertical split: top 40% / bottom 60% (`Flex::SpaceBetween`, 1-row spacing).
- Top row: four horizontal columns (`Min(22)` each): Calendar, Weather, Daily Picture, Dictionary.
- **Left column** is a single framed panel (`theme::frame_block("📅 Calendar")`) owned by the Dashboard. Inside, the greeting and month grid are laid out responsively: when the panel is wide enough (inner width ≥ `GREETING_MIN_WIDTH + MONTHLY_WIDTH` = 49), `Greeting` sits on the left and the `Calendar` grid goes at the top-right; on narrower terminals they stack — greeting on top, grid below (right-aligned) — so the datetime line isn't clipped. With four top columns the calendar column is often narrow enough to stack. The grid itself is right-aligned within whatever area it receives (`Calendar::draw`). Below the panel sits `Inspiration` (`Length(6)`).
- Remaining top columns: `Weather`, `Daily Picture` (photo via `ratatui_image::StatefulImage` (`Resize::Fit`) with a 2-row hint line below showing `Shift+N`), and `Dictionary` (search input + definitions).
- **Bottom 60%** is split horizontally into `Wikipedia` (`Ratio(1, 3)`, left) and `News` (`Ratio(2, 3)`, right) with 1-col spacing (`Flex::SpaceBetween`). Wikipedia uses the taller pane for search results (snippets + word counts) and a larger extract preview (Wikidata description + lead extract).

Greeting and Calendar no longer coordinate hardcoded offsets — the Dashboard owns the frame and hands each widget a `Layout`-derived sub-`Rect`.

### Widgets

- **Greeting** (`src/components/greeting.rs`) — username greeting, live clock/datetime, and IP-based location. Source of the shared location state consumed by Weather.
- **Calendar** (`src/components/calendar.rs`) — monthly calendar with today highlighted; grid placed at the top-right of the area the Dashboard allocates.
- **Weather** (`src/components/weather.rs`) — current conditions and a 7-day forecast bar chart via Open-Meteo. Reads location from the Greeting's shared state (does **not** create its own Greeting).
- **Inspiration** (`src/components/inspiration.rs`) — daily inspirational quote via ZenQuotes.
- **Dictionary** (`src/components/dictionary.rs`) — interactive word lookup via the Free Dictionary API; `ratatui-textarea` input with inline definition rendering.
- **Wikipedia** (`src/components/wikipedia.rs`) — on-demand Wikipedia search via the MediaWiki Action API (`list=search`), then a **single** follow-up `prop=extracts|description` request that fills every result. Arrow-key selection is local (no per-row HTTP); results use ratatui `List` + `ListState` (same idea as News's `TableState`). `/` enters editing mode (Dictionary keeps `Esc` to start typing); Esc leaves editing; Enter submits the query (or opens the selected article when the input still matches the last search). While editing, `↑`/`↓` move the selection. Registered **before** Dictionary and News in `Dashboard::components()` so Esc-to-exit Wikipedia wins over Dictionary's Esc-to-edit. Normal mode only listens for `/`, so Dictionary's Enter-to-search is never stolen.
- **News** (`src/components/news.rs`) — scrollable, categorized news table with keyboard navigation and browser links.
- **Daily Picture** (`src/components/picture_frame.rs`) — a random photo from Lorem Picsum's `/{w}/{h}` endpoint (which serves a different image per request; no API key, no rate limit) rendered via `ratatui-image` (auto-detects kitty/iTerm2/sixel; falls back to unicode halfblocks). Uses `StatefulImage` + `ThreadProtocol` so resize/encode is offloaded off the render path. The panel title is the static "🖼 Daily Picture"; no per-image metadata is fetched or shown, and a one-line hint below the image advertises `Shift+N`. It fetches once on startup, then only on demand: pressing `Shift+N` sets `ImageState.refetch_requested`, and the next `maybe_spawn_fetch` grabs a fresh random photo (deferred if a fetch is already in flight). A failed fetch is not auto-retried — press `Shift+N` to try again (the last-good image keeps showing if one was already loaded). The handler matches on the uppercase `N` character (exactly what Shift+N produces), so lowercase `n` and any `Ctrl`/`Ctrl+Shift` combo (reported by terminals as lowercase `Ctrl+n`) are ignored. On resize, `App::handle_resize` calls `tui.resize()` (ratatui's `Terminal::resize`), which clears the viewport (`ESC[2J`) and resets ratatui's back buffer so the next render repaints everything. (It must **not** call `Terminal::clear`, which does a blocking `ESC[6n` cursor-position round-trip that our background `EventStream` starves, timing out with "The cursor position could not be read within a normal duration".) Note that `ESC[2J` doesn't reliably evict images already _placed_ by a graphics protocol (iTerm2/kitty/sixel), so a resize can still leave an on-screen ghost; pressing `Shift+N` re-fetches and re-installs the protocol (`replace_protocol`), which re-places the image cleanly. A `Clear` widget in the draw would _not_ help — ratatui resets the frame buffer every pass (`swap_buffers`), and the ghost lives on the terminal surface, not in the buffer. Override the protocol with `DAILY_DASHBOARD_IMAGE_PROTOCOL=auto|halfblocks|kitty|sixel|iterm2`; Warp is forced to `iTerm2` (it renders the iTerm2 OSC 1337 inline-image protocol but not Kitty Unicode placeholders, which would emit `[?]` tofu); the VS Code/Cursor integrated terminal (`TERM_PROGRAM=vscode`) is forced to `halfblocks` because its inline-image support (`terminal.integrated.enableImages`) is off by default yet it still answers graphics capability queries, so a graphics protocol would render nothing (override with `DAILY_DASHBOARD_IMAGE_PROTOCOL=iterm2` if you enabled that setting); use `halfblocks` for any other terminal that emits `[?]` tofu (e.g. inside `tmux`).
- **FPS** (`src/components/fps.rs`) — performance counter (disabled / dead code).

### Event System

- Actions are defined in `src/action.rs` (Tick, Render, Resize, Quit, Suspend, Resume, Error, …).
- Components communicate via `tokio::sync::mpsc` unbounded channels; the main loop in `src/app.rs` dispatches actions to `Dashboard`, which fans them out to children.
- Configurable keybindings in `config.json5` map keys to actions per `Mode` (only `Home` exists).
- **Event propagation:** `Dashboard::handle_events` first delivers events to components with `is_capturing_input()` (Dictionary / Wikipedia while editing), then to the rest. That way typing `/` in the Dictionary isn't stolen by Wikipedia's `/`-to-edit, while Wikipedia can still sit before Dictionary in `components()` so Esc-to-exit Wikipedia wins over Esc-to-edit Dictionary when neither (or Wikipedia) is capturing. Propagation **stops** as soon as one returns `Some(action)`.
- **Two key paths:** global keys (`q`, `Ctrl-c`, `Ctrl-d`, `Ctrl-z`) go through the config keymap → `Action`. Widget-specific keys (News navigation, Dictionary/Wikipedia input) are handled directly in each widget's `handle_events` and bypass the keymap.

### State Management

- Widget state uses `Arc<Mutex<T>>` (`std::sync`) for thread-safe shared state. There are no true concurrent readers (the render thread and the fetch task never hold a lock at the same time meaningfully), so `Mutex` is used over `RwLock` for a simpler, single-`lock()` mental model. Note `Mutex` is **not reentrant**: never call a helper that locks the same state while already holding its guard (this is why `news::draw` reads the selected index from the guard it holds instead of calling a re-locking helper).
- `LoadingStatus` enum (`src/app.rs`) tracks async states: `NotStarted`, `Loading`, `Loaded`, `Error(String)`. Fetch/parse failures go through `LoadingStatus::from_report(prefix, &err)` (for `color_eyre::Report`) or `LoadingStatus::from_msg(prefix, msg)` (for plain strings): both log once under `prefix` and build the UI `Error` string with Display formatting (`{e}` / `{e:#}` in logs for eyre chains — never `{e:?}`).
- Async data fetching uses `tokio::spawn` inside `update()` on `Action::Tick` (or on submit for the Dictionary / Wikipedia). The `TextArea` is not `Send`, so the Dictionary and Wikipedia keep it outside the `Arc<Mutex<…>>` they share with their spawn tasks.
- **Weather depends on location data from the Greeting component's shared state** — `Dashboard` constructs `Greeting` first and passes `greeting.state.clone()` into `Weather::new(...)`, so there is exactly one location fetch on startup.
- **Daily Picture** keeps the non-`Clone` `ratatui_image::ThreadProtocol` (the `StatefulImage` state) as a direct field, outside the `Arc<Mutex<ImageState>>` it shares with its fetch task. The fetch task only clones `client` + `state` and decodes bytes to an `image::DynamicImage` (stored as `state.pending_image`); the main thread then creates the protocol from the `Picker` in `update()` (`install_pending_image`). The `Picker` is built once in `Dashboard::new()` (i.e. inside `App::new()`, **before** `tui.enter()` starts the event loop) via `Picker::from_query_stdio()` so its stdin query doesn't race crossterm's `EventStream`, falling back to `Picker::halfblocks()` on terminals with no graphics protocol.

### HTTP & Data Fetching

- `src/http.rs` builds one shared `reqwest::Client` (10s timeout, descriptive `daily-dashboard/<version> (<repo>; <email>)` user-agent required by Wikimedia's UA policy, pooled connections). Cloning a `Client` is cheap, so the single client is cloned into each fetching widget instead of being rebuilt per request.
- `http::get_json(url)`, `http::get_text(url)`, and `http::get_bytes_redirected(url)` handle the GET → `error_for_status` → body decode → JSON / text / raw bytes ladder, returning `color_eyre::Result`. Every widget fetch (greeting, weather, news, inspiration, dictionary, wikipedia) goes through these helpers; the Daily Picture widget uses `get_bytes_redirected` for the image file and decodes it with `image::load_from_memory`.
- IP **geolocation** uses the free [ip-api.com](https://ip-api.com/) JSON endpoint via `http::get_json` (after resolving the public IP from ipify / ifconfig.me / icanhazip).

### Theming

- `src/theme.rs` centralizes the visual language so widgets don't hardcode styles:
  - `panel_block(title)` / `panel_block_colored(title, color)` — bordered block drawn **before** content (content renders into `block.inner(area)`); uses `.style()` so the accent reaches empty cells.
  - `frame_block(title)` — bordered block drawn **after** content (the shared Calendar panel); uses only `.border_style()` / `.title_style()` (no `.style()`) so it doesn't recolor already-drawn cells.
  - Color constants: `ACCENT` (Cyan, borders/titles), `LOADING` (Yellow), `ERROR` (Red), `HINT` (DarkGray).

### Refresh Intervals

- **Greeting / Location**: fetched once on startup.
- **Weather**: every 10 minutes (after location is loaded). Failed fetches retry after 1 minute.
- **News**: every 30 minutes. Failed fetches retry after 1 minute. Overlapping fetches are gated by setting `Loading` before spawn.
- **Inspiration**: once on the first tick (daily quote). Failed fetches retry after 1 minute (not every tick).
- **Daily Picture**: once on startup, then only on demand via `Shift+N` (a fresh random Lorem Picsum photo each time). `Shift+N` sets `ImageState.refetch_requested`, honored by the next `maybe_spawn_fetch` (deferred if a fetch is already in flight). A failed fetch is not auto-retried; press `Shift+N` to retry (the last-good image keeps showing if one was already loaded).
- **Dictionary**: on demand, when the user submits a word.
- **Wikipedia**: on demand, when the user submits a search (`/`, type, Enter). Exactly two HTTP calls per search (search + one batched extracts query); changing selection does not hit the network.

## Key Dependencies

- **ratatui** 0.30.1 — TUI framework (`widget-calendar` feature for the month grid)
- **ratatui-image** 11.0 — image rendering for the Daily Picture widget (kitty/iTerm2/sixel/halfblocks; `crossterm` + `tokio` features enabled, `chafa` and `image-defaults` disabled to avoid the native `libchafa` dependency and to control image formats ourselves)
- **image** 0.25 — image decoding (`jpeg`/`png`/`webp`/`gif`) for the Daily Picture widget
- **ratatui-textarea** 0.9.2 — editable text input for the Dictionary and Wikipedia
- **crossterm** 0.29.0 — terminal input/output (with `event-stream`)
- **tokio** 1.40.0 — async runtime (full features)
- **reqwest** 0.13.4 — HTTP client (`json` feature) for all API calls
- **serde** / **serde_json** / **json5** — (de)serialization + JSON5 config
- **clap** 4.5.20 — CLI argument parsing
- **config** 0.15.23 — layered configuration with JSON5 support
- **color-eyre** 0.6.3 — error reporting (the app standardizes on `color_eyre::Result`)
- **tracing** 0.1.40 / **tracing-subscriber** — structured logging
- **chrono** 0.4 / **time** 0.3.41 — date/time handling
- **open** 5.0 — open URLs in the default browser (News, Wikipedia)
- **whoami** 2.1.2 — current username (Greeting)

## Configuration System

Configuration is layered:

1. Embedded defaults from `.config/config.json5` (compiled into the binary).
2. User overrides from the config directory (`config.json5`, `config.json`, `config.yaml`, `config.toml`, or `config.ini`).

Supports:

- Keybindings per mode (`Home`).
- Style configuration with color parsing.
- Environment variable overrides (`DAILY_DASHBOARD_CONFIG`, `DAILY_DASHBOARD_DATA`, `DAILY_DASHBOARD_IMAGE_PROTOCOL`).

Default keybindings (config-driven, global):

- `q`, `Ctrl-d`, `Ctrl-c` — Quit
- `Ctrl-z` — Suspend (returns to shell)

Widget keys (handled directly in each widget, **not** via the config keymap):

- **News**: `i` / `Up` — move selection up · `j` / `Down` — move selection down · `Enter` — open the selected article in the default browser.
- **Dictionary**: `Esc` — enter editing mode (or leave it) · type to edit · `Enter` — look up the word. While editing, the Dictionary consumes all keypresses so News doesn't react.
- **Wikipedia**: `/` — enter editing mode · `Esc` — leave editing · type to edit · `↑`/`↓` — move selection while editing · `Enter` — search, or open the selected article when the input still matches the last query. While editing, Wikipedia consumes all keys so Dictionary/News don't also react. Normal mode only handles `/` (so Dictionary's Enter-to-search is never stolen).
- **Daily Picture**: `Shift+N` — fetch a new random photo.

## Data Sources & APIs

| Widget        | Source                                                                                                           | Notes                                                                                                                                    |
| ------------- | ---------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| Weather       | [Open-Meteo](https://open-meteo.com/)                                                                            | No API key. Current conditions + 7-day forecast.                                                                                         |
| Location      | Public IP (ipify / ifconfig.me / icanhazip) → [ip-api.com](https://ip-api.com/)                                  | IP tried from multiple endpoints; geolocation via `http::get_json` to ip-api.                                                            |
| Inspiration   | [ZenQuotes](https://zenquotes.io/api/today)                                                                      | Daily quote (`q` text, `a` author).                                                                                                      |
| Dictionary    | [Free Dictionary API](https://api.dictionaryapi.dev/api/v2/entries/en)                                           | Word → phonetic + meanings/definitions. 404 on unknown words is surfaced as an error.                                                    |
| Wikipedia     | [MediaWiki Action API](https://www.mediawiki.org/wiki/API:Search) (`list=search` + `prop=extracts\|description`) | Search then one batched extracts request for all hits. Enter opens the desktop article URL. No API key. Compliant User-Agent required.   |
| News          | [ok.surf](https://ok.surf/api/v1/cors/news-feed)                                                                 | Business, Technology, Sports, Politics, Health, Entertainment.                                                                           |
| Daily Picture | [Lorem Picsum](https://picsum.photos)                                                                            | No API key, no rate limit. Random photo from `/<w>/<h>` (a different image per request); fetched on startup and on-demand via `Shift+N`. |

## Testing Notes

- Tests live in `src/tests/*.rs` and are attached per-module with `#[cfg(test)] #[path = "../tests/<name>.rs"] mod tests;`, giving them access to private items via `use super::*;`.
- **Pure parse functions** are extracted out of the fetch paths and unit-tested:
  - `news::parse_articles` — category ordering, missing-field skipping, per-category cap, unknown/empty categories.
  - `weather::parse_daily_forecast` — weekday/temp extraction, missing `daily`, bad dates, partial arrays.
  - `dictionary::parse_entry` and `dictionary::build_definition_text` — word/phonetic/meaning extraction, phonetics-array fallback, empty-meaning skipping, rendered text.
  - `greeting::parse_location` — ip-api success/fail status handling, field extraction, missing-field defaults.
  - `inspiration::parse_quote` — ZenQuotes array shape, missing `q`/`a`, empty text.
  - `picture_frame::image_url` and `picture_frame::is_new_image_key` — Lorem Picsum random-URL construction, and the Shift+N (uppercase `N`) key detection that ignores lowercase `n` and `Ctrl`/`Ctrl+Shift` combos.
  - `wikipedia::parse_search_results`, `wikipedia::strip_search_html`, `wikipedia::parse_extracts_query`, `wikipedia::apply_extracts`, `wikipedia::is_search_activation_key`, and `wikipedia::enter_action` — MediaWiki search parsing, HTML snippet stripping, batched extracts/descriptions, `/` activation, and Enter search-vs-open.
- **Render snapshot tests** use `ratatui::backend::TestBackend` + `Terminal` to draw a widget into a buffer and assert on the visible text (see `src/tests/inspiration.rs`).
- **HTTP helper tests** (`src/tests/http.rs`) exercise `http::shared_client`, `get_text`, `get_json`, and `get_bytes_redirected` against a tiny in-process HTTP/1.1 server bound to an ephemeral localhost port (no external network, so they run in the default hermetic suite). Covers 200/404 response handling, JSON parse success/failure, and redirect-following with final-URL capture.
- **Live-network tests** are marked `#[ignore]` so the default suite is hermetic; run them with `cargo test -- --ignored`.
- Lint and doc gates are strict: `clippy --all-targets -- -D warnings` and `cargo doc --document-private-items` with `RUSTDOCFLAGS=-D warnings`.

## CI/CD

- **CI** (`.github/workflows/ci.yml`): on push to `main` / PRs, runs tests, `rustfmt --check`, clippy (`-D warnings`), and docs (`-D warnings`) on nightly.
- **CD** (`.github/workflows/cd.yml`): cross-platform release builds and `cargo publish` on version tags.
