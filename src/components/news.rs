use crate::{action::Action, app::LoadingStatus, components::Component, tui::Event};
use chrono::Local;
use color_eyre::eyre::ErrReport;
use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Cell, Row, Table, TableState},
};
use std::sync::{Arc, RwLock};
use tracing::error;

const NEW_API_URL: &str = "https://ok.surf/api/v1/cors/news-feed";
const MAX_NUMBER_OF_ARTICLES_FROM_EACH_CATEGORY: usize = 30;
const MAX_NUMBER_OF_ARTICLES: usize = 200;
const FETCH_INTERVAL_MINS: i64 = 30;
const NEWS_CATEGORIES: [&str; 6] = [
    "Business",
    "Technology",
    "Sports",
    "Politics",
    "Health",
    "Entertainment",
];

#[derive(Clone, Debug)]
pub struct NewsArticle {
    pub title: String,
    pub link: String,
    pub source: String,
    pub category: String,
}

#[derive(Clone, Debug, Default)]
pub struct NewsState {
    pub loading_status: LoadingStatus,
    pub news_articles: Vec<NewsArticle>,
    pub last_updated_at: Option<chrono::DateTime<Local>>,
    pub table_state: TableState,
}

#[derive(Clone, Debug)]
pub struct News {
    state: Arc<RwLock<NewsState>>,
}

impl News {
    pub fn new() -> Self {
        let mut table_state: TableState = TableState::default();
        table_state.select(Some(0)); // Have the first article always selected

        Self {
            state: Arc::new(RwLock::new(NewsState {
                table_state,
                ..Default::default()
            })),
        }
    }

    fn set_loading_state(&self, status: LoadingStatus) {
        let mut state = self.state.write().unwrap();
        state.loading_status = status;
    }

    async fn fetch_news_data(&self) {
        self.set_loading_state(LoadingStatus::Loading);

        let api_url = NEW_API_URL.to_string();
        let response = match reqwest::get(&api_url).await {
            Ok(resp) => resp,
            Err(e) => {
                let error_msg = format!("API request failed: {e:?}");
                error!("News: {error_msg:?}");
                self.set_loading_state(LoadingStatus::Error(error_msg));
                return;
            }
        };

        if !response.status().is_success() {
            let error_msg = format!("API returned error status: {}", response.status());
            error!("News: {}", error_msg);
            self.set_loading_state(LoadingStatus::Error(error_msg));
            return;
        }

        let body_text = match response.text().await {
            Ok(text) => text,
            Err(e) => {
                let error_msg = format!("Failed to read response body: {e:?}",);
                error!("News: {error_msg:?}");
                self.set_loading_state(LoadingStatus::Error(error_msg));
                return;
            }
        };

        let json: serde_json::Value = match serde_json::from_str(&body_text) {
            Ok(json) => json,
            Err(e) => {
                let error_msg = format!("Failed to parse JSON: {e:?}");
                error!("News: {error_msg:?}");
                self.set_loading_state(LoadingStatus::Error(error_msg));
                return;
            }
        };

        let mut articles: Vec<NewsArticle> = Vec::new();
        for category in NEWS_CATEGORIES {
            if let Some(values) = json.get(category)
                && let Some(array) = values.as_array()
            {
                let category_articles: Vec<NewsArticle> = array
                    .iter()
                    .take(MAX_NUMBER_OF_ARTICLES_FROM_EACH_CATEGORY)
                    .filter_map(|article| {
                        let article_object = article.as_object()?;
                        Some(NewsArticle {
                            title: article_object
                                .get("title")?
                                .as_str()?
                                .trim_matches('"')
                                .to_string(),
                            link: article_object
                                .get("link")?
                                .as_str()?
                                .trim_matches('"')
                                .to_string(),
                            source: article_object
                                .get("source")?
                                .as_str()?
                                .trim_matches('"')
                                .to_string(),
                            category: category.to_string(),
                        })
                    })
                    .collect();
                articles.extend(category_articles);
            }
        }

        articles.truncate(MAX_NUMBER_OF_ARTICLES);

        if articles.is_empty() {
            let error_msg = "No articles found in response".to_string();
            error!("News: {}", error_msg);
            self.set_loading_state(LoadingStatus::Error(error_msg));
            return;
        }

        let mut news_state = self.state.write().unwrap();
        news_state.news_articles = articles;
        news_state.last_updated_at = Some(Local::now());
        news_state.loading_status = LoadingStatus::Loaded;
    }

    fn get_selected_row_fg_color(&self, idx: usize) -> Style {
        let news_state = self.state.read().unwrap();
        let mut selected_color = Style::default().fg(Color::White);
        if let Some(selected_idx) = news_state.table_state.selected()
            && selected_idx == idx
        {
            selected_color = Style::default().fg(Color::Black);
        };
        selected_color
    }
}

impl Component for News {
    fn handle_events(&mut self, event: Option<Event>) -> color_eyre::Result<Option<Action>> {
        match event {
            Some(Event::Key(key_event)) => match key_event.code {
                KeyCode::Char('i') | KeyCode::Up => {
                    let mut state = self.state.write().unwrap();
                    let selected = state.table_state.selected().unwrap_or(0);
                    if selected > 0 {
                        state.table_state.select(Some(selected - 1));
                    }
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    let mut state = self.state.write().unwrap();
                    let selected = state.table_state.selected().unwrap_or(0);
                    let max_index = state.news_articles.len().saturating_sub(1);
                    if selected < max_index {
                        state.table_state.select(Some(selected + 1));
                    }
                }
                KeyCode::Enter => {
                    let state = self.state.read().unwrap();
                    if let Some(selected) = state.table_state.selected()
                        && let Some(article) = state.news_articles.get(selected)
                    {
                        let url = article.link.trim_matches('"');
                        if let Err(e) = open::that(url) {
                            error!("News: Failed to open URL {}: {}", url, e);
                        }
                    }
                }
                _ => {}
            },
            Some(Event::Mouse(_mouse_event)) => {}
            _ => (),
        };
        Ok(None)
    }

    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
        if action == Action::Tick {
            let should_fetch = {
                let news_state = self.state.read().unwrap();
                let is_initial_load = matches!(
                    news_state.loading_status,
                    LoadingStatus::NotStarted | LoadingStatus::Error(_)
                );
                let now = Local::now();
                let should_refresh = match news_state.last_updated_at {
                    Some(last_updated) => {
                        let duration = now.signed_duration_since(last_updated);
                        duration.num_minutes() >= FETCH_INTERVAL_MINS
                    }
                    None => true,
                };
                is_initial_load || should_refresh
            };

            if should_fetch {
                let this = self.clone();
                tokio::spawn(async move {
                    this.fetch_news_data().await;
                });
            }
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<(), ErrReport> {
        let news_state_read = self.state.read().unwrap();
        match &news_state_read.loading_status {
            LoadingStatus::NotStarted => {
                let block = Block::default()
                    .title("News")
                    .style(Style::default().fg(Color::Yellow));
                frame.render_widget(block, area);
            }
            LoadingStatus::Loading => {
                let block = Block::default()
                    .title("News - Loading...")
                    .style(Style::default().fg(Color::Yellow));
                frame.render_widget(block, area);
            }
            LoadingStatus::Error(error) => {
                let block = Block::default()
                    .title(format!("News - Error: {error}"))
                    .style(Style::default().fg(Color::Red));
                frame.render_widget(block, area);
            }
            LoadingStatus::Loaded => {
                let last_updated = news_state_read
                    .last_updated_at
                    .map(|dt| dt.format("%H:%M").to_string())
                    .unwrap_or_else(|| "Unknown".to_string());

                let title = format!(
                    "News ({} articles) - Updated: {}",
                    news_state_read.news_articles.len(),
                    last_updated
                );

                let header = Row::new(vec![
                    Cell::from(Span::styled(
                        "Title",
                        Style::default().add_modifier(Modifier::BOLD),
                    )),
                    Cell::from(Span::styled(
                        "Source",
                        Style::default().add_modifier(Modifier::BOLD),
                    )),
                    Cell::from(Span::styled(
                        "Category",
                        Style::default().add_modifier(Modifier::BOLD),
                    )),
                ])
                .style(Style::default().fg(Color::Yellow))
                .height(1);

                let rows: Vec<Row> = news_state_read
                    .news_articles
                    .iter()
                    .enumerate()
                    .map(|(idx, article)| {
                        Row::new(vec![
                            Cell::from(article.title.clone()),
                            Cell::from(article.source.clone()),
                            Cell::from(article.category.clone()),
                        ])
                        .style(self.get_selected_row_fg_color(idx))
                        .height(1)
                    })
                    .collect();

                let table = Table::new(
                    rows,
                    [
                        ratatui::layout::Constraint::Percentage(80),
                        ratatui::layout::Constraint::Percentage(10),
                        ratatui::layout::Constraint::Percentage(10),
                    ],
                )
                .header(header)
                .block(
                    Block::default()
                        .title(Line::from(title).centered().style(Style::default().dim()))
                        .style(Style::default().fg(Color::Yellow)),
                )
                .row_highlight_style(Style::default().bg(Color::White))
                .highlight_symbol("📌 ");

                let news_area = Rect {
                    x: area.x + 2,
                    y: area.y + 2,
                    width: area.width.saturating_sub(4),
                    height: area.height.saturating_sub(3),
                };

                drop(news_state_read); // Release the read lock first
                let mut table_state_write = self.state.write().unwrap().table_state;
                frame.render_stateful_widget(table, news_area, &mut table_state_write);
            }
        }
        Ok(())
    }
}
