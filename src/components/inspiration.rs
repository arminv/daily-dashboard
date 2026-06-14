use super::Component;
use crate::{action::Action, app::LoadingStatus};
use color_eyre::Result;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph, Wrap},
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
        let state = self.state.read().unwrap();

        let inspiration_area = Rect {
            x: area.x + 2,
            y: area.y + 1,
            width: area.width,
            height: area.height,
        };

        match &state.loading_status {
            LoadingStatus::NotStarted => {
                let block = Block::default()
                    .title("Daily Inspiration")
                    .style(Style::default().fg(Color::Yellow));
                frame.render_widget(block, inspiration_area);
            }
            LoadingStatus::Loading => {
                let block = Block::default()
                    .title("Daily Inspiration - Loading...")
                    .style(Style::default().fg(Color::Yellow));
                frame.render_widget(block, inspiration_area);
            }
            LoadingStatus::Error(error) => {
                let block = Block::default()
                    .title(format!("Daily Inspiration - Error: {error}"))
                    .style(Style::default().fg(Color::Red));
                frame.render_widget(block, inspiration_area);
            }
            LoadingStatus::Loaded => {
                let quote_line = Line::from(vec![
                    Span::styled("❝", Style::default().fg(Color::Cyan)),
                    Span::styled(
                        state.quote_text.clone(),
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled("❞", Style::default().fg(Color::Cyan)),
                ]);

                let author_line = Line::from(vec![
                    Span::styled("— ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        state.quote_author.clone(),
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::ITALIC),
                    ),
                ]);

                let paragraph = Paragraph::new(vec![quote_line, author_line])
                    .style(Style::new().cyan())
                    .block(Block::default().title("Daily Inspiration"))
                    .wrap(Wrap { trim: true });

                frame.render_widget(paragraph, inspiration_area);
            }
        }

        Ok(())
    }
}
