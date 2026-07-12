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
use serde_json::Value;
use std::sync::{
    Arc,
    Mutex,
};
use tracing::error;

const NEWS_API_URL: &str = "https://ok.surf/api/v1/cors/news-feed";
const MAX_NUMBER_OF_ARTICLES_FROM_EACH_CATEGORY: usize = 30;
const MAX_NUMBER_OF_ARTICLES: usize = 200;
const FETCH_INTERVAL_MINS: i64 = 30;
const RETRY_NEWS_ON_ERROR_IN_MINS: i64 = 1;
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
    state: Arc<Mutex<NewsState>>,
    client: reqwest::Client,
}

impl News {
    pub fn new(client: reqwest::Client) -> Self {
        let mut table_state: TableState = TableState::default();
        table_state.select(Some(0)); // Have the first article always selected

        Self {
            state: Arc::new(Mutex::new(NewsState {
                table_state,
                ..Default::default()
            })),
            client,
        }
    }

    fn set_error_state(&self, status: LoadingStatus) {
        let mut state = self.state.lock().unwrap();
        state.loading_status = status;
        state.last_updated_at = Some(Local::now());
    }

    fn move_selection(&self, delta: i32) {
        let mut state = self.state.lock().unwrap();
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
        let json = match http::get_json(&self.client, NEWS_API_URL).await {
            Ok(json) => json,
            Err(e) => {
                self.set_error_state(LoadingStatus::from_report("News", &e));
                return;
            }
        };

        let articles = parse_articles(&json);
        if articles.is_empty() {
            self.set_error_state(LoadingStatus::from_msg(
                "News",
                "no articles found in response",
            ));
            return;
        }

        let mut news_state = self.state.lock().unwrap();
        news_state.news_articles = articles;
        news_state.last_updated_at = Some(Local::now());
        news_state.loading_status = LoadingStatus::Loaded;
    }
}

fn parse_articles(json: &Value) -> Vec<NewsArticle> {
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
                    let state = self.state.lock().unwrap();
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
                let mut news_state = self.state.lock().unwrap();
                let is_stale = |mins: i64| {
                    news_state.last_updated_at.is_none_or(|last| {
                        Local::now().signed_duration_since(last).num_minutes() >= mins
                    })
                };
                let should_fetch = match news_state.loading_status {
                    LoadingStatus::NotStarted => true,
                    LoadingStatus::Loading => false,
                    LoadingStatus::Loaded => is_stale(FETCH_INTERVAL_MINS),
                    LoadingStatus::Error(_) => is_stale(RETRY_NEWS_ON_ERROR_IN_MINS),
                };
                if should_fetch {
                    news_state.loading_status = LoadingStatus::Loading;
                }
                should_fetch
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
        let mut news_state = self.state.lock().unwrap();
        match &news_state.loading_status {
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
                let last_updated = news_state
                    .last_updated_at
                    .map(|dt| dt.format("%H:%M").to_string())
                    .unwrap_or_else(|| "Unknown".to_string());

                let title = format!(
                    "📰 News ({} articles) · Updated: {}",
                    news_state.news_articles.len(),
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

                let selected = news_state.table_state.selected();
                let rows: Vec<Row> = news_state
                    .news_articles
                    .iter()
                    .enumerate()
                    .map(|(idx, article)| {
                        let fg = if selected == Some(idx) {
                            Color::Black
                        } else {
                            Color::White
                        };
                        Row::new(vec![
                            Cell::from(article.title.clone()),
                            Cell::from(article.source.clone()),
                            Cell::from(article.category.clone()),
                        ])
                        .style(Style::default().fg(fg))
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

                frame.render_stateful_widget(table, area, &mut news_state.table_state);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "../tests/news.rs"]
mod tests;
