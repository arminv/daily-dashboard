use std::time::Duration;

use color_eyre::{Result, eyre::WrapErr};
use serde_json::Value;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const USER_AGENT: &str = concat!("daily-dashboard/", env!("CARGO_PKG_VERSION"));

/// Build the single HTTP client shared by every widget.
///
/// Cloning a [`reqwest::Client`] is cheap (it is internally `Arc`-backed) and
/// reuses the same connection pool, so callers should clone this one client and
/// pass it around instead of building a new client per request.
pub fn shared_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .user_agent(USER_AGENT)
        .build()
        .wrap_err("failed to build HTTP client")
}

/// GET `url`, require a 2xx response, and parse the body as JSON.
pub async fn get_json(client: &reqwest::Client, url: &str) -> Result<Value> {
    let body = get_text(client, url).await?;
    serde_json::from_str(&body).wrap_err_with(|| format!("failed to parse JSON from {url}"))
}

/// GET `url`, require a 2xx response, and return the body as text.
pub async fn get_text(client: &reqwest::Client, url: &str) -> Result<String> {
    let response = client
        .get(url)
        .send()
        .await
        .wrap_err_with(|| format!("request failed for {url}"))?;
    let response = response
        .error_for_status()
        .wrap_err_with(|| format!("HTTP error from {url}"))?;
    response
        .text()
        .await
        .wrap_err_with(|| format!("failed to read response body from {url}"))
}
