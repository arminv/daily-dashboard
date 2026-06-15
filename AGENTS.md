# AGENTS.md

This file provides guidance to AI agents when working with code in this repository.

## Project Overview

Daily Dashboard is a Terminal User Interface (TUI) application written in Rust that displays weather, news, calendar, and greetings in a dashboard format. The application uses `ratatui` for the TUI framework and follows an async, component-based architecture.

## Essential Commands

### Build & Run

```bash
# Build the project
cargo build

# Run with default settings (4 ticks/sec, 60 frames/sec)
cargo run

# Run with custom tick/frame rates
cargo run -- --tick-rate 2.0 --frame-rate 30.0

# Run tests
cargo test

# Check for linting issues
cargo clippy

# Format code
cargo fmt
```

### Development

- Main entry point: `src/main.rs`
- Default config (embedded): `.config/config.json5` (JSON5 format with comments)
- User config directory: platform-specific via `directories` crate (override with `DAILY_DASHBOARD_CONFIG`)
- Logs: `logs.log` file for debugging

## Architecture Overview

### Application Flow

1. `main.rs` initializes error handling and logging, parses CLI args, and runs `App`
2. `App` (`src/app.rs`) owns the event loop, config, and a list of top-level `Component` trait objects
3. `Tui` (`src/tui.rs`) wraps ratatui/crossterm and runs a background event loop (tick, render, keyboard, resize)
4. Actions flow through `tokio::sync::mpsc` channels between the TUI layer and components

### Component System

The application uses a trait-based component architecture where each widget implements the `Component` trait defined in `src/components.rs`. Components must implement:

- `draw()` - Render the component (required)
- `update()` - React to actions (optional override)
- `handle_events()` - Process input events (optional override)
- `register_action_handler()` - Register action callbacks (optional override)
- `register_config_handler()` - Receive config (optional override)
- `init()` - Initialize with terminal size (optional override)

### Dashboard Composite

`src/dashboard.rs` is the only top-level component registered in `App`. It composes and lays out all widgets:

```
┌─────────────────────────────────────────┐
│  Top 20%                                │
│  ┌──────────────────┬─────────────────┐ │
│  │ Calendar         │ Weather         │ │
│  │ + Greeting       │ (7-day forecast)│ │
│  └──────────────────┴─────────────────┘ │
├─────────────────────────────────────────┤
│  Bottom 80%: News feed table            │
└─────────────────────────────────────────┘
```

Calendar and Greeting share the top-left area (overlapping layout). Dashboard delegates `handle_events` and `update` to child components.

### Widgets

- **Greeting** (`src/components/greeting.rs`) - Username greeting, live clock, and IP-based location
- **Calendar** (`src/components/calendar.rs`) - Monthly calendar with today highlighted
- **Weather** (`src/components/weather.rs`) - Current conditions and 7-day forecast bar chart via Open-Meteo
- **News** (`src/components/news.rs`) - Scrollable news table with keyboard navigation and browser links
- **FPS** (`src/components/fps.rs`) - Performance counter (fully commented out / disabled)

### Event System

- Actions are defined in `src/action.rs` (Tick, Render, Resize, Quit, Suspend, Resume, etc.)
- Components communicate via `tokio::sync::mpsc` unbounded channels
- Main event loop in `src/app.rs` dispatches actions to components
- Configurable keybindings in `config.json5` map keys to actions per `Mode`

### State Management

- Widget state uses `Arc<RwLock<T>>` for thread-safe shared state
- `LoadingStatus` enum (`src/app.rs`) tracks async operation states: `NotStarted`, `Loading`, `Loaded`, `Error(String)`
- Async data fetching uses `tokio::spawn` inside `update()` on `Action::Tick`
- Weather depends on location data from the Greeting component's shared state

### Refresh Intervals

- **Greeting/Location**: Fetched once on startup
- **Weather**: Every 10 minutes (after location is loaded)
- **News**: Every 30 minutes

## Key Dependencies

- **ratatui** 0.30.1 - TUI framework with calendar widget support
- **crossterm** 0.29.0 - Terminal input/output
- **tokio** 1.40.0 - Async runtime (full features enabled)
- **reqwest** 0.13.4 - HTTP client for API calls
- **clap** 4.5.20 - CLI argument parsing
- **config** 0.15.23 - Configuration management with JSON5 support
- **color-eyre** 0.6.3 - Enhanced error reporting
- **tracing** 0.1.40 - Structured logging
- **ipgeolocate** 0.3 - IP-based geolocation
- **chrono** / **time** - Date/time handling
- **open** 5.0 - Open URLs in the default browser

## Configuration System

Configuration is layered:

1. Embedded defaults from `.config/config.json5` (compiled into the binary)
2. User overrides from the config directory (`config.json5`, `config.json`, `config.yaml`, `config.toml`, or `config.ini`)

Supports:

- Keybindings per mode (`Home`)
- Style configuration with color parsing
- Environment variable overrides (`DAILY_DASHBOARD_CONFIG`, `DAILY_DASHBOARD_DATA`)

Default keybindings:

- `q`, `Ctrl-d`, `Ctrl-c` - Quit
- `Ctrl-z` - Suspend (returns to shell)

News widget keys (handled directly in the News component, not via config):

- `i` / `Up` - Move selection up
- `j` / `Down` - Move selection down
- `Enter` - Open selected article in default browser

## Data Sources & APIs

- **Weather**: [Open-Meteo API](https://open-meteo.com/) (no API key required)
- **Location**: Public IP via ipify/ifconfig.me/icanhazip, then geolocation via `ipgeolocate` (ip-api.com)
- **News**: [ok.surf news feed API](https://ok.surf/api/v1/cors/news-feed) (Business, Technology, Sports, Politics, Health, Entertainment categories)

## Testing Notes

- Config module has comprehensive test coverage (key parsing, style parsing, config loading)
- Test with `cargo test`
- Use `cargo clippy` for linting (project follows clippy recommendations)
- CI runs tests, rustfmt, clippy, and docs on every push/PR to `main`

## CI/CD

- **CI** (`.github/workflows/ci.yml`): Tests, formatting, clippy, and docs on push/PR to `main`
- **CD** (`.github/workflows/cd.yml`): Cross-platform release builds and `cargo publish` on version tags
