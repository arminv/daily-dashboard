use color_eyre::{
    Result,
    eyre::WrapErr,
};
use serde_json::Value;
use std::time::Duration;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
/// Wikimedia requires a descriptive UA with contact info (URL and/or email).
/// A bare `name/version` string is treated as poorly identified and rate-limits hard.
/// See <https://foundation.wikimedia.org/wiki/Policy:Wikimedia_Foundation_User-Agent_Policy>.
const USER_AGENT: &str = concat!(
    "daily-dashboard/",
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("CARGO_PKG_REPOSITORY"),
    "; arminvarshokar@gmail.com)"
);

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
    let status = response.status();
    let response = response
        .error_for_status()
        .wrap_err_with(|| format!("HTTP {status} from {url}"))?;
    response
        .text()
        .await
        .wrap_err_with(|| format!("failed to read response body from {url}"))
}

/// GET `url`, follow redirects, require a 2xx response, and return the raw body
/// bytes together with the final (post-redirect) URL.
///
/// Used for binary downloads (e.g. fetching an image file to decode). The final
/// URL is returned because Lorem Picsum redirects `/seed/<seed>/...` to a
/// canonical `/id/<id>/...` image URL whose `id` is needed for the metadata
/// lookup.
pub async fn get_bytes_redirected(
    client: &reqwest::Client,
    url: &str,
) -> Result<(Vec<u8>, String)> {
    let response = client
        .get(url)
        .send()
        .await
        .wrap_err_with(|| format!("request failed for {url}"))?;
    let status = response.status();
    let final_url = response.url().to_string();
    let response = response
        .error_for_status()
        .wrap_err_with(|| format!("HTTP {status} from {url}"))?;
    response
        .bytes()
        .await
        .map(|bytes| bytes.to_vec())
        .map(|bytes| (bytes, final_url))
        .wrap_err_with(|| format!("failed to read response body from {url}"))
}

#[cfg(test)]
#[path = "tests/http.rs"]
mod tests;
