use super::Component;
use crate::{
    action::Action,
    app::LoadingStatus,
    http,
    theme,
};
use chrono::Local;
use color_eyre::{
    Result,
    eyre::{
        WrapErr,
        bail,
        eyre,
    },
};
use ratatui::{
    Frame,
    layout::Rect,
    style::{
        Color,
        Modifier,
        Style,
    },
    text::{
        Line,
        Span,
    },
    widgets::{
        Paragraph,
        Wrap,
    },
};
use serde_json::Value;
use std::sync::{
    Arc,
    Mutex,
};

const QUOTE_API_URL: &str = "https://zenquotes.io/api/today";
const RETRY_INSPIRATION_ON_ERROR_IN_MINS: i64 = 1;

#[derive(Clone, Debug, Default)]
pub struct InspirationState {
    pub loading_status: LoadingStatus,
    pub quote_text: String,
    pub quote_author: String,
    pub last_updated_at: Option<chrono::DateTime<Local>>,
}

#[derive(Clone, Debug)]
pub struct Inspiration {
    state: Arc<Mutex<InspirationState>>,
    client: reqwest::Client,
}

impl Inspiration {
    pub fn new(client: reqwest::Client) -> Self {
        Self {
            state: Arc::new(Mutex::new(InspirationState::default())),
            client,
        }
    }

    fn set_error_state(&self, status: LoadingStatus) {
        let mut state = self.state.lock().unwrap();
        state.loading_status = status;
        state.last_updated_at = Some(Local::now());
    }

    async fn fetch_daily_content(&self) {
        let (quote_text, quote_author) = match self.fetch_quote().await {
            Ok(quote) => quote,
            Err(e) => {
                self.set_error_state(LoadingStatus::from_report("Inspiration", &e));
                return;
            }
        };

        let mut state = self.state.lock().unwrap();
        state.quote_text = quote_text;
        state.quote_author = quote_author;
        state.loading_status = LoadingStatus::Loaded;
        state.last_updated_at = Some(Local::now());
    }

    async fn fetch_quote(&self) -> Result<(String, String)> {
        let json = http::get_json(&self.client, QUOTE_API_URL)
            .await
            .wrap_err("failed to fetch quote")?;
        parse_quote(&json)
    }
}

/// Parse a ZenQuotes `/api/today` JSON payload into `(text, author)`. Pure (no
/// I/O) so it can be unit-tested.
fn parse_quote(json: &Value) -> Result<(String, String)> {
    let entry = json
        .as_array()
        .and_then(|arr| arr.first())
        .ok_or_else(|| eyre!("unexpected quote response format"))?;

    let quote_text = entry
        .get("q")
        .and_then(|v| v.as_str())
        .ok_or_else(|| eyre!("missing quote text"))?
        .to_string();

    let quote_author = entry
        .get("a")
        .and_then(|v| v.as_str())
        .ok_or_else(|| eyre!("missing quote author"))?
        .to_string();

    if quote_text.is_empty() {
        bail!("empty quote text");
    }

    Ok((quote_text, quote_author))
}

/// Render a bordered status panel (title + one-line message) for the
/// NotStarted / Loading / Error states of the inspiration widget.
fn render_status(
    frame: &mut Frame,
    area: Rect,
    title: String,
    color: Color,
    message: String,
    message_color: Color,
) {
    let block = theme::panel_block_colored(title, color);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(
        Paragraph::new(message)
            .style(Style::default().fg(message_color))
            .wrap(Wrap { trim: true }),
        inner,
    );
}

impl Component for Inspiration {
    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        if action == Action::Tick {
            let should_fetch = {
                let mut state = self.state.lock().unwrap();
                let is_stale = |mins: i64| {
                    state.last_updated_at.is_none_or(|last| {
                        Local::now().signed_duration_since(last).num_minutes() >= mins
                    })
                };
                let should_fetch = match state.loading_status {
                    LoadingStatus::NotStarted => true,
                    LoadingStatus::Loading => false,
                    LoadingStatus::Loaded => false,
                    LoadingStatus::Error(_) => is_stale(RETRY_INSPIRATION_ON_ERROR_IN_MINS),
                };
                if should_fetch {
                    state.loading_status = LoadingStatus::Loading;
                }
                should_fetch
            };

            if should_fetch {
                let this = self.clone();
                tokio::spawn(async move {
                    this.fetch_daily_content().await;
                });
            }
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let inspiration_state = self.state.lock().unwrap();

        match &inspiration_state.loading_status {
            LoadingStatus::NotStarted => render_status(
                frame,
                area,
                "✨ Daily Quote".to_string(),
                theme::ACCENT,
                "Fetching today's quote...".to_string(),
                theme::HINT,
            ),
            LoadingStatus::Loading => render_status(
                frame,
                area,
                "✨ Daily Quote — Loading...".to_string(),
                theme::LOADING,
                "Fetching today's quote...".to_string(),
                theme::HINT,
            ),
            LoadingStatus::Error(error) => render_status(
                frame,
                area,
                format!("✨ Daily Quote — Error: {error}"),
                theme::ERROR,
                format!("Couldn't load today's quote: {error}"),
                Color::LightRed,
            ),
            LoadingStatus::Loaded => {
                let quote_line = Line::from(vec![
                    Span::styled("❝", Style::default().fg(theme::ACCENT)),
                    Span::styled(
                        inspiration_state.quote_text.clone(),
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled("❞", Style::default().fg(theme::ACCENT)),
                ]);
                let author_line = Line::from(vec![
                    Span::styled("— ", Style::default().fg(theme::HINT)),
                    Span::styled(
                        inspiration_state.quote_author.clone(),
                        Style::default()
                            .fg(theme::HINT)
                            .add_modifier(Modifier::ITALIC),
                    ),
                ]);
                let paragraph = Paragraph::new(vec![quote_line, author_line])
                    .block(theme::panel_block("✨ Daily Quote"))
                    .style(Style::new().cyan())
                    .wrap(Wrap { trim: true });

                frame.render_widget(paragraph, area);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "../tests/inspiration.rs"]
mod tests;
