# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Daily Dashboard is a Terminal User Interface (TUI) application written in Rust that displays weather, news, calendar, and greetings in a dashboard format. The application uses `ratatui` for the TUI framework and follows an async, component-based architecture.

## Essential Commands

### Build & Run
```bash
# Build the project
cargo build

# Run with default settings
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
- Configuration: `.config/config.json5` (JSON5 format with comments)
- Logs: `logs.log` file for debugging

## Architecture Overview

### Component System
The application uses a trait-based component architecture where each widget implements the `Component` trait defined in `src/components.rs`. Components must implement:
- `init()` - Initialize component state
- `draw()` - Render the component
- `update()` - Update component state
- `handle_events()` - Process input events
- `register_action_handler()` - Register action callbacks

### Current Components
- **Greeting** (`src/components/greeting.rs`) - User greeting and location data
- **Weather** (`src/components/weather.rs`) - Weather display with 7-day forecast using Open-Meteo API
- **Calendar** (`src/components/calendar.rs`) - Calendar widget
- **News** (`src/components/news.rs`) - News feed with keyboard navigation
- **FPS** (`src/components/fps.rs`) - Performance counter (currently disabled)

### Event System
- Actions are defined in `src/action.rs` (Tick, Render, Resize, Quit, etc.)
- Components communicate via `tokio::sync::mpsc` channels
- Main event loop in `src/app.rs` dispatches actions to components

### State Management
- Uses `Arc<RwLock<T>>` for thread-safe shared state between components
- `LoadingStatus` enum tracks async operation states (NotStarted, Loading, Loaded, Error)
- Weather component depends on location data from greeting component

## Key Dependencies

- **ratatui** 0.29.0 - TUI framework with calendar widget support
- **tokio** 1.40.0 - Async runtime (full features enabled)
- **reqwest** 0.11 - HTTP client for API calls
- **clap** 4.5.20 - CLI argument parsing
- **config** 0.14.0 - Configuration management with JSON5 support
- **color-eyre** 0.6.3 - Enhanced error reporting
- **tracing** 0.1.40 - Structured logging

## Configuration System

Configuration is stored in `.config/config.json5` and supports:
- Keybindings per mode (Insert, Normal)
- Style configuration with color parsing
- Frame rate and tick rate settings
- Environment variable overrides

## Data Sources & APIs

- **Weather**: Open-Meteo API (no API key required)
- **Location**: IP-based geolocation via `ipgeolocate` crate
- **News**: Implementation varies by news source

## Testing Notes

- Config module has comprehensive test coverage
- Test with `cargo test`
- Use `cargo clippy` for linting (project follows clippy recommendations)

## Current Development

- Active branch: `news-widget` (working on news feed functionality)
- Weather widget includes 7-day forecast with color-coded temperature bars
- Auto-refresh every 10 minutes for weather data
- Keyboard navigation system implemented for news widget