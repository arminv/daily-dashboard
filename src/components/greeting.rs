use super::Component;
use crate::{
    app::LoadingStatus,
    http,
    theme,
};
use chrono::Local;
use color_eyre::Result;
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
use tracing::error;

const IP_API_URLS: [&str; 3] = [
    "https://api.ipify.org",
    "https://ifconfig.me/ip",
    "https://icanhazip.com",
];

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

        match self.get_public_ip().await {
            Ok(ip) => {
                let service = ipgeolocate::Service::IpApi;

                match ipgeolocate::Locator::get(&ip, service).await {
                    Ok(ip_info) => {
                        let location_data = LocationState {
                            city: ip_info.city,
                            country: ip_info.country,
                            latitude: ip_info.latitude.parse().unwrap_or(0.0),
                            longitude: ip_info.longitude.parse().unwrap_or(0.0),
                            timezone: ip_info.timezone,
                        };
                        let mut state = self.state.lock().unwrap();
                        state.location = location_data;
                        state.loading_status = LoadingStatus::Loaded;
                    }
                    Err(error) => {
                        error!("Error fetching IP location: {}", error);
                        self.set_loading_state(LoadingStatus::Error(error.to_string()));
                    }
                }
            }
            Err(error) => {
                error!("Error fetching public IP: {}", error);
                self.set_loading_state(LoadingStatus::Error(format!(
                    "Failed to get public IP: {error}",
                )));
            }
        }
    }

    async fn get_public_ip(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // Try multiple IP API services in case one fails
        for api_url in IP_API_URLS {
            match http::get_text(&self.client, api_url).await {
                Ok(ip) => {
                    let ip = ip.trim().to_string();
                    if !ip.is_empty() {
                        return Ok(ip);
                    }
                }
                Err(_) => continue,
            }
        }
        Err("Failed to get public IP".into())
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
