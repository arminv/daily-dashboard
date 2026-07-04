use super::Component;
use crate::{
    action::Action,
    app::LoadingStatus,
    http,
    theme,
};
use color_eyre::Result;
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
use std::sync::{
    Arc,
    RwLock,
};
use tracing::error;

const QUOTE_API_URL: &str = "https://zenquotes.io/api/today";

#[derive(Clone, Debug, Default)]
pub struct InspirationState {
    pub loading_status: LoadingStatus,
    pub quote_text: String,
    pub quote_author: String,
}

#[derive(Clone, Debug)]
pub struct Inspiration {
    state: Arc<RwLock<InspirationState>>,
    client: reqwest::Client,
}

impl Inspiration {
    pub fn new(client: reqwest::Client) -> Self {
        Self {
            state: Arc::new(RwLock::new(InspirationState::default())),
            client,
        }
    }

    fn set_loading_state(&self, status: LoadingStatus) {
        let mut state = self.state.write().unwrap();
        state.loading_status = status;
    }

    async fn fetch_daily_content(&self) {
        self.set_loading_state(LoadingStatus::Loading);

        let (quote_text, quote_author) = match self.fetch_quote().await {
            Ok(quote) => quote,
            Err(e) => {
                let error_msg = format!("Failed to fetch quote: {e}");
                error!("Inspiration (Quote): {error_msg}");
                self.set_loading_state(LoadingStatus::Error(error_msg));
                return;
            }
        };

        let mut state = self.state.write().unwrap();
        state.quote_text = quote_text;
        state.quote_author = quote_author;
        state.loading_status = LoadingStatus::Loaded;
    }

    async fn fetch_quote(&self) -> Result<(String, String), String> {
        let json = http::get_json(&self.client, QUOTE_API_URL)
            .await
            .map_err(|e| format!("Failed to fetch quote: {e}"))?;

        let entry = json
            .as_array()
            .and_then(|arr| arr.first())
            .ok_or("Unexpected quote response format")?;

        let quote_text = entry
            .get("q")
            .and_then(|v| v.as_str())
            .ok_or("Missing quote text")?
            .to_string();

        let quote_author = entry
            .get("a")
            .and_then(|v| v.as_str())
            .ok_or("Missing quote author")?
            .to_string();

        Ok((quote_text, quote_author))
    }
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
                let state = self.state.read().unwrap();
                let is_initial_load = matches!(
                    state.loading_status,
                    LoadingStatus::NotStarted | LoadingStatus::Error(_)
                );
                is_initial_load
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
        let inspiration_state_read = self.state.read().unwrap();

        match &inspiration_state_read.loading_status {
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
                        inspiration_state_read.quote_text.clone(),
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled("❞", Style::default().fg(theme::ACCENT)),
                ]);

                let author_line = Line::from(vec![
                    Span::styled("— ", Style::default().fg(theme::HINT)),
                    Span::styled(
                        inspiration_state_read.quote_author.clone(),
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
