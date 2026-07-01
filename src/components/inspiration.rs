use super::Component;
use crate::{action::Action, app::LoadingStatus};
use color_eyre::Result;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use std::sync::{Arc, RwLock};
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
}

impl Inspiration {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(InspirationState::default())),
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
        let response = reqwest::get(QUOTE_API_URL)
            .await
            .map_err(|e| format!("API request failed: {e:?}"))?;

        if !response.status().is_success() {
            return Err(format!("API returned error status: {}", response.status()));
        }

        let body_text = response
            .text()
            .await
            .map_err(|e| format!("Failed to read response body: {e:?}"))?;

        let json: serde_json::Value =
            serde_json::from_str(&body_text).map_err(|e| format!("Failed to parse JSON: {e:?}"))?;

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

        let block = |title: String, color: Color| {
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .style(Style::default().fg(color))
        };

        match &inspiration_state_read.loading_status {
            LoadingStatus::NotStarted => {
                let b = block("✨ Daily Inspiration".to_string(), Color::Cyan);
                let inner = b.inner(area);
                frame.render_widget(b, area);
                frame.render_widget(
                    Paragraph::new("Fetching today's quote...")
                        .style(Style::default().fg(Color::DarkGray))
                        .wrap(Wrap { trim: true }),
                    inner,
                );
            }
            LoadingStatus::Loading => {
                let b = block(
                    "✨ Daily Inspiration — Loading...".to_string(),
                    Color::Yellow,
                );
                let inner = b.inner(area);
                frame.render_widget(b, area);
                frame.render_widget(
                    Paragraph::new("Fetching today's quote...")
                        .style(Style::default().fg(Color::DarkGray))
                        .wrap(Wrap { trim: true }),
                    inner,
                );
            }
            LoadingStatus::Error(error) => {
                let b = block(format!("✨ Daily Inspiration — Error: {error}"), Color::Red);
                let inner = b.inner(area);
                frame.render_widget(b, area);
                frame.render_widget(
                    Paragraph::new(format!("Couldn't load today's quote: {error}"))
                        .style(Style::default().fg(Color::LightRed))
                        .wrap(Wrap { trim: true }),
                    inner,
                );
            }
            LoadingStatus::Loaded => {
                let quote_line = Line::from(vec![
                    Span::styled("❝", Style::default().fg(Color::Cyan)),
                    Span::styled(
                        inspiration_state_read.quote_text.clone(),
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled("❞", Style::default().fg(Color::Cyan)),
                ]);

                let author_line = Line::from(vec![
                    Span::styled("— ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        inspiration_state_read.quote_author.clone(),
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::ITALIC),
                    ),
                ]);

                let paragraph = Paragraph::new(vec![quote_line, author_line])
                    .block(block("✨ Daily Inspiration".to_string(), Color::Cyan))
                    .style(Style::new().cyan())
                    .wrap(Wrap { trim: true });

                frame.render_widget(paragraph, area);
            }
        }
        Ok(())
    }
}
