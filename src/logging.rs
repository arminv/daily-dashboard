use color_eyre::Result;
use tracing_error::ErrorLayer;
use tracing_subscriber::{
    EnvFilter,
    fmt,
    prelude::*,
};

use crate::config;

lazy_static::lazy_static! {
    pub static ref LOG_ENV: String = format!("{}_LOG_LEVEL", config::PROJECT_NAME.clone());
}

pub fn init() -> Result<()> {
    // Write next to the process cwd (typically the repo root under `cargo run`),
    // matching the gitignored `/logs.log` path.
    let log_file = std::fs::File::create("logs.log")?;
    let env_filter = EnvFilter::builder().with_default_directive(tracing::Level::INFO.into());
    // If the `RUST_LOG` environment variable is set, use that as the default, otherwise use the
    // value of the `LOG_ENV` environment variable. If the `LOG_ENV` environment variable contains
    // errors, then this will return an error.
    let env_filter = env_filter
        .try_from_env()
        .or_else(|_| env_filter.with_env_var(LOG_ENV.clone()).from_env())?;
    let file_subscriber = fmt::layer()
        .with_file(true)
        .with_line_number(true)
        .with_writer(log_file)
        .with_target(false)
        .with_ansi(false)
        .with_filter(env_filter);
    tracing_subscriber::registry()
        .with(file_subscriber)
        .with(ErrorLayer::default())
        .try_init()?;
    Ok(())
}
