use crate::app::LoadingStatus;
use crate::components::Component;
use crate::{action::Action, tui::Event};
use chrono::Local;
use color_eyre::eyre::ErrReport;
use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::widgets::TableState;
use ratatui::Frame;
use std::sync::{Arc, RwLock};
use tracing::{error, info};

#[derive(Clone, Debug)]
pub struct NewsArticle {
    pub title: String,
    pub link: String,
    pub source: String,
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
        Self {
            state: Arc::new(RwLock::new(NewsState::default())),
        }
    }

    fn set_loading_state(&self, status: LoadingStatus) {
        let mut state = self.state.write().unwrap();
        state.loading_status = status;
    }

    async fn fetch_news_data(&self) {
        self.set_loading_state(LoadingStatus::Loading);
        let api_url = "https://ok.surf/api/v1/cors/news-feed".to_string();
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

        // Get the response text
        let body_text = match response.text().await {
            Ok(text) => text,
            Err(e) => {
                let error_msg = format!("Failed to read response body: {e:?}",);
                error!("News: {error_msg:?}");
                self.set_loading_state(LoadingStatus::Error(error_msg));
                return;
            }
        };

        // Parse the JSON
        let json: serde_json::Value = match serde_json::from_str(&body_text) {
            Ok(json) => json,
            Err(e) => {
                let error_msg = format!("Failed to parse JSON: {e:?}");
                error!("News: {error_msg:?}");
                self.set_loading_state(LoadingStatus::Error(error_msg));
                return;
            }
        };

        // info!("News: Parsed JSON: {:?}", json);

        // Extract the weather data
        let current = match json.get("Business") {
            // TODO:
            Some(current) => current,
            None => {
                let error_msg = "No 'Business' field in response".to_string();
                error!("News: {}", error_msg);
                self.set_loading_state(LoadingStatus::Error(error_msg));
                return;
            }
        };

        // TODO:
        // let mut news_state = self.state.write().unwrap();
        // news_state.news_articles = current;

        info!(
            "News: Current:{:?} ------ {:?}",
            current,
            self.state.read().unwrap()
        );
        //
        // if let Some(code) = current.get("weather_code")
        //     && let Some(value) = code.as_u64()
        // {
        //     let code_value = value as u32;
        //     news_state.description = self.get_weather_description(code_value);
        //     news_state.icon = self.get_weather_icon(code_value);
        //     info!("Weather: Code {}: {}", value, news_state.description);
        // }
    }
}

impl Component for News {
    fn handle_events(&mut self, event: Option<Event>) -> color_eyre::Result<Option<Action>> {
        match event {
            Some(Event::Key(key_event)) => match key_event.code {
                KeyCode::Char('i') | KeyCode::Up => {
                    info!("News: Scrolling up!");
                    self.state.write().unwrap().table_state.scroll_up_by(1)
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    info!("News: Scrolling down!");
                    self.state.write().unwrap().table_state.scroll_down_by(1)
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
            // let should_fetch = {
            //     let weather_state = self.state.read().unwrap();
            //
            //     // Check if weather needs initial loading or had an error
            //     let is_initial_load = matches!(
            //         weather_state.loading_status,
            //         LoadingStatus::NotStarted | LoadingStatus::Error(_)
            //     );
            //
            //     // Check if 5 minutes have passed since the last update
            //     let now = Local::now();
            //     let should_refresh = match weather_state.last_updated_at {
            //         Some(last_updated) => {
            //             let duration = now.signed_duration_since(last_updated);
            //             // Refresh if more than n minutes have passed
            //             duration.num_minutes() >= 10
            //         }
            //         None => true, // No previous update, so fetch
            //     };
            //
            //     is_initial_load || should_refresh
            // };

            // if should_fetch {
            //     let this = self.clone();
            //     tokio::spawn(async move {
            //         info!("Weather: Fetching weather data");
            //         this.fetch_weather_data().await;
            //     });
            // }

            let this = self.clone();
            tokio::spawn(async move {
                info!("News: Fetching news data");
                this.fetch_news_data().await;
            });
        }

        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<(), ErrReport> {
        Ok(())
    }
}
