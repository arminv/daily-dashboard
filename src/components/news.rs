use crate::{
    action::Action,
    app::LoadingStatus,
    components::Component,
    http,
    theme,
    tui::Event,
};
use chrono::Local;
use crossterm::event::KeyCode;
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
        Cell,
        Row,
        Table,
        TableState,
    },
};
use std::sync::{
    Arc,
    RwLock,
};
use tracing::error;

const NEWS_API_URL: &str = "https://ok.surf/api/v1/cors/news-feed";
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
    client: reqwest::Client,
}

impl News {
    pub fn new(client: reqwest::Client) -> Self {
        let mut table_state: TableState = TableState::default();
        table_state.select(Some(0)); // Have the first article always selected

        Self {
            state: Arc::new(RwLock::new(NewsState {
                table_state,
                ..Default::default()
            })),
            client,
        }
    }

    fn set_loading_state(&self, status: LoadingStatus) {
        let mut state = self.state.write().unwrap();
        state.loading_status = status;
    }

    fn move_selection(&self, delta: i32) {
        let mut state = self.state.write().unwrap();
        let len = state.news_articles.len();
        if len == 0 {
            return;
        }
        let current = state.table_state.selected().unwrap_or(0);
        let max_index = len - 1;
        let next = (current as i32 + delta).clamp(0, max_index as i32) as usize;
        state.table_state.select(Some(next));
    }

    async fn fetch_news_data(&self) {
        self.set_loading_state(LoadingStatus::Loading);

        let json = match http::get_json(&self.client, NEWS_API_URL).await {
            Ok(json) => json,
            Err(e) => {
                let error_msg = format!("News: {e}");
                error!("{error_msg}");
                self.set_loading_state(LoadingStatus::Error(error_msg));
                return;
            }
        };

        let articles = parse_articles(&json);
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

/// Parse the ok.surf news-feed JSON into a flat list of articles, capped at
/// [`MAX_NUMBER_OF_ARTICLES_FROM_EACH_CATEGORY`] per category and
/// [`MAX_NUMBER_OF_ARTICLES`] overall. Pure (no I/O) so it can be unit-tested.
fn parse_articles(json: &serde_json::Value) -> Vec<NewsArticle> {
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
    articles
}

impl Component for News {
    fn handle_events(&mut self, event: Option<Event>) -> color_eyre::Result<Option<Action>> {
        if let Some(Event::Key(key)) = event {
            match key.code {
                KeyCode::Char('i') | KeyCode::Up => self.move_selection(-1),
                KeyCode::Char('j') | KeyCode::Down => self.move_selection(1),
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
            }
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

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        let news_state_read = self.state.read().unwrap();
        match &news_state_read.loading_status {
            LoadingStatus::NotStarted => {
                frame.render_widget(theme::panel_block("📰 News"), area);
            }
            LoadingStatus::Loading => {
                frame.render_widget(theme::panel_block("📰 News — Loading..."), area);
            }
            LoadingStatus::Error(error) => {
                frame.render_widget(
                    theme::panel_block_colored(format!("📰 News — Error: {error}"), theme::ERROR),
                    area,
                );
            }
            LoadingStatus::Loaded => {
                let last_updated = news_state_read
                    .last_updated_at
                    .map(|dt| dt.format("%H:%M").to_string())
                    .unwrap_or_else(|| "Unknown".to_string());

                let title = format!(
                    "📰 News ({} articles) · Updated: {}",
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
                .block(theme::panel_block(
                    Line::from(title).centered().style(Style::default().dim()),
                ))
                .row_highlight_style(Style::default().bg(Color::White))
                .highlight_symbol("📌 ");

                drop(news_state_read); // Release the read lock first
                let mut table_state_write = self.state.write().unwrap().table_state;
                frame.render_stateful_widget(table, area, &mut table_state_write);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "../tests/news.rs"]
mod tests;
