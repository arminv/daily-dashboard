use super::Component;
use crate::{
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
    },
};
use ratatui::{
    Frame,
    layout::{
        Constraint,
        Direction,
        Layout,
        Rect,
    },
    style::{
        Color,
        Modifier,
        Style,
    },
    widgets::Paragraph,
};
use std::sync::{
    Arc,
    Mutex,
};
use tracing::debug;

const IP_API_URLS: [&str; 3] = [
    "https://api.ipify.org",
    "https://ifconfig.me/ip",
    "https://icanhazip.com",
];
const GEO_API_URL: &str =
    "http://ip-api.com/json/{ip}?fields=status,message,city,country,lat,lon,timezone";

#[derive(Debug, Clone, Default)]
pub struct LocationState {
    pub city: String,
    pub country: String,
    pub latitude: f64,
    pub longitude: f64,
    pub timezone: String,
}

#[derive(Debug, Default, Clone)]
pub struct GreetingState {
    pub location: LocationState,
    pub loading_status: LoadingStatus,
}

#[derive(Debug, Default, Clone)]
pub struct Greeting {
    pub state: Arc<Mutex<GreetingState>>,
    client: reqwest::Client,
}

impl Greeting {
    pub fn new(client: reqwest::Client) -> Self {
        let greeting = Self {
            client,
            ..Self::default()
        };
        greeting.run();
        greeting
    }

    fn run(&self) {
        let this = self.clone();
        tokio::spawn(async move {
            this.fetch_location_data().await;
        });
    }

    fn set_loading_state(&self, status: LoadingStatus) {
        let mut state = self.state.lock().unwrap();
        state.loading_status = status;
    }

    async fn fetch_location_data(&self) {
        self.set_loading_state(LoadingStatus::Loading);

        let ip = match self.get_public_ip().await {
            Ok(ip) => ip,
            Err(e) => {
                self.set_loading_state(LoadingStatus::from_report("Greeting", &e));
                return;
            }
        };

        let url = GEO_API_URL.replace("{ip}", &ip);
        let json = match http::get_json(&self.client, &url)
            .await
            .wrap_err("failed to fetch IP location")
        {
            Ok(json) => json,
            Err(e) => {
                self.set_loading_state(LoadingStatus::from_report("Greeting", &e));
                return;
            }
        };

        match parse_location(&json) {
            Ok(location_data) => {
                let mut state = self.state.lock().unwrap();
                state.location = location_data;
                state.loading_status = LoadingStatus::Loaded;
            }
            Err(e) => {
                self.set_loading_state(LoadingStatus::from_report("Greeting", &e));
            }
        }
    }

    async fn get_public_ip(&self) -> Result<String> {
        for api_url in IP_API_URLS {
            match http::get_text(&self.client, api_url).await {
                Ok(ip) => {
                    let ip = ip.trim().to_string();
                    if !ip.is_empty() {
                        return Ok(ip);
                    }
                    debug!("Greeting: empty response from {api_url}");
                }
                Err(e) => {
                    debug!("Greeting: IP provider {api_url} failed: {e:#}");
                }
            }
        }
        bail!("all public-IP providers failed");
    }

    fn get_location_display(&self) -> String {
        let state = self.state.lock().unwrap();
        let location_loading = "🌐 Location is loading...".to_string();

        match state.loading_status {
            LoadingStatus::NotStarted => location_loading,
            LoadingStatus::Loading => location_loading,
            LoadingStatus::Loaded => {
                format!("🌐 {}, {}", state.location.city, state.location.country,)
            }
            LoadingStatus::Error(ref error) => format!("Location error: {error}"),
        }
    }
}

impl Component for Greeting {
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let greeting_message = String::from("👋 ")
            + &whoami::username()
                .unwrap_or("User".to_string())
                .to_uppercase();
        let now = Local::now();
        let datetime_str = now.format("%a, %b %d, %Y %H:%M:%S").to_string();
        let location_str = self.get_location_display();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(area);

        frame.render_widget(
            Paragraph::new(datetime_str).style(Style::default().fg(theme::ACCENT)),
            chunks[0],
        );
        frame.render_widget(
            Paragraph::new(greeting_message).style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::White),
            ),
            chunks[1],
        );
        frame.render_widget(
            Paragraph::new(location_str).style(Style::default().fg(Color::Green)),
            chunks[2],
        );
        Ok(())
    }
}

fn parse_location(json: &serde_json::Value) -> Result<LocationState> {
    let status = json.get("status").and_then(|v| v.as_str()).unwrap_or("");
    if status != "success" {
        let message = json
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error");
        bail!("ip-api lookup failed: {message}");
    }

    Ok(LocationState {
        city: json
            .get("city")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        country: json
            .get("country")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        latitude: json.get("lat").and_then(|v| v.as_f64()).unwrap_or(0.0),
        longitude: json.get("lon").and_then(|v| v.as_f64()).unwrap_or(0.0),
        timezone: json
            .get("timezone")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
    })
}

#[cfg(test)]
#[path = "../tests/greeting.rs"]
mod tests;
